use std::collections::HashMap;

use anyhow::Context;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use bollard::Docker;
use garde::Validate;
use crate::routing::api::validation::not_empty;
use serde::Deserialize;
use slasha_db::{
    DbPool, DbResult,
    app::{AppSource, NewApp},
    deployment::{Deployment, DeploymentStatus},
    git_connection::NewGitConnection,
    github_connection::{ConnectionStatus, NewGithubConnection},
    repos::{
        app::{AppRepo, NewAppConnection},
        app_domain::AppDomainRepo,
        app_scale::AppScaleRepo,
        deployment::DeploymentRepo,
        git_connection::GitConnectionRepo,
        github_connection::GithubConnectionRepo,
        service::ServiceRepo,
    },
};
use tokio::process::Command;
use uuid::Uuid;

use crate::{
    HttpError, HttpResult,
    connections::{
        GithubError, sync_external_app, sync_selected_git_repository,
        sync_selected_github_repository,
    },
    docker::{
        deployment::{remove_app_images, remove_app_volumes, remove_deployment_processes},
        network::{create_app_network, remove_app_network},
        service::remove_service_container,
    },
    extractors::{
        ValidatedJson,
        app::{ActiveApp, ActiveAppOwner},
        auth::AuthUser,
    },
    routing::api::deserialize::trim_string,
    state::{AppState, Config, Runtime, Storage},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_app))
        .route("/", get(list_apps))
        .route("/check-slug", get(check_slug))
        .route("/{slug}", get(get_app))
        .route("/{slug}/connection", get(get_connection))
        .route("/{slug}/scales", get(list_scales))
        .route("/{slug}", delete(delete_app))
        .route("/{slug}/settings", put(update_settings))
        .route(
            "/{slug}/connection/github",
            put(reconnect_github).delete(disconnect_github),
        )
        .route("/{slug}/connection/branch", put(update_connection_branch))
        .route("/{slug}/sync", post(sync_app))
}

#[derive(Deserialize, Validate)]
struct CreateAppReq {
    #[serde(deserialize_with = "trim_string")]
    #[garde(custom(not_empty))]
    name: String,
    #[garde(skip)]
    #[serde(flatten)]
    source: CreateAppSource,
}

#[derive(Deserialize)]
#[serde(tag = "source", rename_all = "lowercase")]
enum CreateAppSource {
    Local,
    Github {
        installation_id: i64,
        repository_id: i64,
        branch: Option<String>,
    },
    Git {
        url: String,
        branch: Option<String>,
    },
}

#[derive(Deserialize)]
struct CheckSlugReq {
    name: String,
}

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

async fn generate_unique_slug(pool: &DbPool, name: &str) -> DbResult<(String, bool)> {
    let base_slug = slugify(name);
    if base_slug.is_empty() {
        return Ok((String::new(), false));
    }

    if !AppRepo::slug_exists(pool, &base_slug).await? {
        return Ok((base_slug, true));
    }

    let mut counter = 1;
    loop {
        let candidate = format!("{}-{}", base_slug, counter);
        if !AppRepo::slug_exists(pool, &candidate).await? {
            return Ok((candidate, false));
        }
        counter += 1;
    }
}

async fn init_local_repository(repo_path: &str) -> HttpResult<()> {
    let output = Command::new("git")
        .arg("init")
        .arg("--bare")
        .arg("--initial-branch=main")
        .arg(repo_path)
        .output()
        .await
        .context("Failed to init bare repo")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("git init --bare failed: {}", stderr).into());
    }
    Ok(())
}

fn validate_public_git_url(value: &str) -> HttpResult<String> {
    let url = reqwest::Url::parse(value)
        .map_err(|_| HttpError::bad_request("Git URL must be a valid HTTP(S) URL"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(HttpError::bad_request(
            "Git URL must be a public HTTP(S) URL without credentials",
        ));
    }
    Ok(url.to_string())
}

async fn prepare_github_connection(
    state: &AppState,
    user_id: &str,
    app_id: &str,
    repo_path: &str,
    installation_id: i64,
    repository_id: i64,
    branch: Option<String>,
) -> HttpResult<(String, NewGithubConnection)> {
    let github = state
        .github_client()
        .await
        .ok_or_else(|| HttpError::not_found("GitHub integration is disabled"))?;

    ensure_github_installation_access(state, user_id, installation_id).await?;
    let repository = sync_selected_github_repository(
        &github,
        &state.runtime,
        app_id,
        std::path::PathBuf::from(repo_path),
        installation_id,
        repository_id,
        branch.clone(),
    )
    .await
    .map_err(|error| {
        HttpError::bad_request(format!("Failed to fetch GitHub repository: {}", error))
    })?;

    let final_branch = branch.unwrap_or(repository.default_branch);

    Ok((
        final_branch,
        NewGithubConnection {
            app_id: app_id.to_string(),
            installation_id,
            repository_id,
            status: ConnectionStatus::Connected,
        },
    ))
}

async fn ensure_github_installation_access(
    state: &AppState,
    user_id: &str,
    installation_id: i64,
) -> HttpResult<()> {
    if !GithubConnectionRepo::user_has_installation(
        &state.storage.db_pool,
        user_id,
        installation_id,
    )
    .await?
    {
        return Err(HttpError::forbidden(
            "GitHub installation is not connected to this user",
        ));
    }

    Ok(())
}

async fn check_slug(
    State(storage): State<Storage>,
    axum::extract::Query(query): axum::extract::Query<CheckSlugReq>,
) -> HttpResult<impl IntoResponse> {
    let (slug, available) = generate_unique_slug(&storage.db_pool, &query.name)
        .await
        .map_err(HttpError::internal)?;

    Ok(Json(serde_json::json!({
        "slug": slug,
        "available": available,
    })))
}

async fn create_app(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    ValidatedJson(payload): ValidatedJson<CreateAppReq>,
) -> HttpResult<impl IntoResponse> {
    let name = payload.name;

    let (slug, _) = generate_unique_slug(&state.storage.db_pool, &name)
        .await
        .map_err(HttpError::internal)?;

    if slug.is_empty() {
        return Err(HttpError::bad_request(
            "App name must contain alphanumeric characters",
        ));
    }

    let repo_path = state
        .storage
        .repos_dir
        .join(format!("{}.git", slug))
        .to_str()
        .ok_or_else(|| HttpError::internal(anyhow::anyhow!("Invalid repo path")))?
        .to_string();

    let app_id = Uuid::new_v4().to_string();
    let (source, default_branch, connection) = match payload.source {
        CreateAppSource::Local => {
            init_local_repository(&repo_path).await?;
            (AppSource::Local, "main".to_string(), None)
        }
        CreateAppSource::Github {
            installation_id,
            repository_id,
            branch,
        } => {
            let (branch, connection) = prepare_github_connection(
                &state,
                &user.id,
                &app_id,
                &repo_path,
                installation_id,
                repository_id,
                branch,
            )
            .await?;
            (
                AppSource::Github,
                branch,
                Some(NewAppConnection::Github(connection)),
            )
        }
        CreateAppSource::Git { url, branch } => {
            let clone_url = validate_public_git_url(&url)?;
            let requested_branch = branch.filter(|branch| !branch.is_empty());
            let branch = sync_selected_git_repository(
                clone_url.clone(),
                requested_branch,
                std::path::PathBuf::from(&repo_path),
            )
            .await
            .map_err(|error| {
                HttpError::bad_request(format!("Failed to fetch Git repository: {error}"))
            })?;
            let connection = NewGitConnection {
                app_id: app_id.clone(),
                clone_url,
            };
            (
                AppSource::Git,
                branch,
                Some(NewAppConnection::Git(connection)),
            )
        }
    };

    let new_app = NewApp {
        id: app_id.clone(),
        slug: slug.clone(),
        name: name.clone(),
        repo_path,
        default_branch,
        auto_deploy: true,
        source,
    };

    let new_app = match connection {
        Some(connection) => {
            AppRepo::create_with_connection(&state.storage.db_pool, new_app, &user.id, connection)
                .await?
        }
        None => AppRepo::create(&state.storage.db_pool, new_app, &user.id).await?,
    };

    create_app_network(&state.clients.docker, &app_id).await?;

    Ok(Json(serde_json::json!({
        "app": new_app,
    })))
}

async fn get_app(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let domains = AppDomainRepo::list_for_app(&state.storage.db_pool, &app.id).await?;
    let url = match domains.first() {
        Some(domain) => format!(
            "https://{}",
            domain
                .domain
                .trim_start_matches("http://")
                .trim_start_matches("https://")
        ),
        None => {
            let scheme = if state.config.platform_domain.contains("localhost") {
                "http"
            } else {
                "https"
            };
            format!("{}://{}.{}", scheme, app.slug, state.config.platform_domain)
        }
    };

    let deployments = DeploymentRepo::list_for_app(&state.storage.db_pool, &app.id).await?;
    let runtime_status = derive_runtime_status(&deployments);

    Ok(Json(serde_json::json!({
        "app": app,
        "url": url,
        "runtime_status": runtime_status,
    })))
}

async fn get_connection(
    State(state): State<AppState>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let connection = match app.source {
        AppSource::Local => None,
        AppSource::Git => GitConnectionRepo::find_for_app(&state.storage.db_pool, &app.id)
            .await?
            .map(|connection| {
                serde_json::json!({
                    "clone_url": connection.clone_url,
                })
            }),
        AppSource::Github => {
            let connection =
                GithubConnectionRepo::find_for_app(&state.storage.db_pool, &app.id).await?;
            match connection {
                Some(connection) => {
                    let repository = if connection.status == ConnectionStatus::Connected {
                        let github = state.github_client().await;
                        match github {
                            Some(github) => match github
                                .get_repository(
                                    connection.installation_id,
                                    connection.repository_id,
                                )
                                .await
                            {
                                Ok(repository) => Some(repository),
                                Err(GithubError::AccessRevoked) => {
                                    GithubConnectionRepo::update_status(
                                        &state.storage.db_pool,
                                        &app.id,
                                        ConnectionStatus::Disconnected,
                                    )
                                    .await?;
                                    None
                                }
                                Err(error) => {
                                    return Err(HttpError::internal(anyhow::Error::from(error)));
                                }
                            },
                            None => None,
                        }
                    } else {
                        None
                    };
                    Some(serde_json::json!({
                        "installation_id": connection.installation_id,
                        "repository_id": connection.repository_id,
                        "repository": repository.map(|repository| serde_json::json!({
                            "full_name": repository.full_name,
                            "html_url": repository.html_url,
                            "default_branch": repository.default_branch,
                        })),
                    }))
                }
                None => None,
            }
        }
    };

    Ok(Json(serde_json::json!({
        "connection": connection,
    })))
}

async fn delete_app(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(runtime): State<Runtime>,
    ActiveAppOwner { app, user: _ }: ActiveAppOwner,
) -> HttpResult<impl IntoResponse> {
    let app_services = ServiceRepo::list_for_app(&db_pool, &app.id).await?;
    let deployments = AppRepo::delete(&db_pool, &app.id).await?;

    let app_clone = app.clone();
    let app_slug = app.slug.clone();
    let app_id = app.id.clone();

    tokio::spawn(async move {
        for service in app_services {
            if let Err(e) =
                remove_service_container(&docker, &runtime.log_manager, &app_clone, &service, true)
                    .await
            {
                tracing::warn!(
                    service_id = %service.id,
                    error = %e,
                    "Failed to delete service"
                );
            }
        }

        for dep in deployments {
            if let Err(e) = remove_deployment_processes(
                &docker,
                &runtime.proxy_sync_trigger,
                &runtime.log_manager,
                &app_clone,
                &dep,
            )
            .await
            {
                tracing::warn!(
                    deployment_id = %dep.id,
                    error = %e,
                    "Failed to remove deployment processes"
                );
            }
        }

        if let Err(e) = remove_app_images(&docker, &app_slug).await {
            tracing::warn!(
                app_slug = %app_slug,
                error = ?e,
                "Failed to remove app images"
            );
        }

        if let Err(e) = remove_app_network(&docker, &app_id).await {
            tracing::warn!(
                app_id = %app_id,
                error = ?e,
                "Failed to remove app network"
            );
        }

        if let Err(e) = remove_app_volumes(&docker, &app_id).await {
            tracing::warn!(
                app_id = %app_id,
                error = ?e,
                "Failed to clean up volumes for app"
            );
        }

        if let Err(e) = runtime.log_manager.delete_app_logs(&app_slug).await {
            tracing::warn!(
                app_slug = %app_slug,
                error = ?e,
                "Failed to delete logs for app"
            );
        }

        let repo_path = std::path::Path::new(&app_clone.repo_path);
        if repo_path.exists()
            && let Err(e) = tokio::fs::remove_dir_all(repo_path).await
        {
            tracing::warn!(
                app_slug = %app_slug,
                error = ?e,
                "Failed to remove repo"
            );
        }
    });

    Ok(Json(serde_json::json!({
        "deleted": true,
        "slug": app.slug,
    })))
}

fn derive_runtime_status(deployments: &[Deployment]) -> &'static str {
    if deployments
        .iter()
        .any(|d| d.status == DeploymentStatus::Running)
    {
        return "running";
    }
    match deployments.first().map(|d| d.status) {
        Some(DeploymentStatus::Building) | Some(DeploymentStatus::Pending) => "deploying",
        Some(DeploymentStatus::Failed) => "failed",
        _ => "idle",
    }
}

async fn list_apps(
    State(storage): State<Storage>,
    State(config): State<Config>,
    AuthUser(user): AuthUser,
) -> HttpResult<impl IntoResponse> {
    let user_apps = AppRepo::list_for_user(&storage.db_pool, &user.id).await?;

    let scheme = if config.platform_domain.contains("localhost") {
        "http"
    } else {
        "https"
    };

    let app_ids = user_apps.iter().map(|app| app.id.clone()).collect();
    let domains = AppDomainRepo::list_for_apps(&storage.db_pool, app_ids).await?;
    let mut primary_domains: HashMap<String, String> = HashMap::new();
    for domain in domains {
        primary_domains
            .entry(domain.app_id)
            .or_insert(domain.domain);
    }

    let mut items = Vec::with_capacity(user_apps.len());
    for app in user_apps {
        let deployments = DeploymentRepo::list_for_app(&storage.db_pool, &app.id).await?;
        let url = match primary_domains.get(&app.id) {
            Some(domain) => format!(
                "https://{}",
                domain
                    .trim_start_matches("http://")
                    .trim_start_matches("https://")
            ),
            None => format!("{}://{}.{}", scheme, app.slug, config.platform_domain),
        };
        items.push(serde_json::json!({
            "app": app,
            "url": url,
            "runtime_status": derive_runtime_status(&deployments),
        }));
    }

    Ok(Json(serde_json::json!({
        "apps": items,
    })))
}

async fn list_scales(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let scales = AppScaleRepo::list_for_app(&storage.db_pool, &app.id).await?;

    Ok(Json(serde_json::json!({ "scales": scales })))
}

#[derive(Deserialize, Validate)]
struct UpdateSettingsReq {
    #[serde(
        default,
        deserialize_with = "crate::routing::api::deserialize::trim_optional_string"
    )]
    #[garde(inner(custom(not_empty)))]
    name: Option<String>,
    #[garde(skip)]
    auto_deploy: Option<bool>,
}

async fn update_settings(
    State(storage): State<Storage>,
    ActiveApp { app, .. }: ActiveApp,
    ValidatedJson(payload): ValidatedJson<UpdateSettingsReq>,
) -> HttpResult<impl IntoResponse> {
    if let Some(auto_deploy) = payload.auto_deploy {
        AppRepo::update_auto_deploy(&storage.db_pool, &app.id, auto_deploy).await?;
    }

    if let Some(name) = payload.name {
        AppRepo::update_name(&storage.db_pool, &app.id, &name).await?;
    }

    Ok(Json(serde_json::json!({
        "success": true,
    })))
}

#[derive(Deserialize, Validate)]
struct ReconnectGithubReq {
    #[garde(skip)]
    installation_id: i64,
    #[garde(skip)]
    repository_id: i64,
}

async fn reconnect_github(
    State(state): State<AppState>,
    ActiveAppOwner { app, user }: ActiveAppOwner,
    ValidatedJson(payload): ValidatedJson<ReconnectGithubReq>,
) -> HttpResult<impl IntoResponse> {
    if app.source != AppSource::Github {
        return Err(HttpError::bad_request("App does not use GitHub"));
    }

    let github = state
        .github_client()
        .await
        .ok_or_else(|| HttpError::not_found("GitHub integration is disabled"))?;
    ensure_github_installation_access(&state, &user.id, payload.installation_id).await?;

    sync_selected_github_repository(
        &github,
        &state.runtime,
        &app.id,
        std::path::PathBuf::from(&app.repo_path),
        payload.installation_id,
        payload.repository_id,
        Some(app.default_branch.clone()),
    )
    .await
    .map_err(|error| {
        HttpError::bad_request(format!("Failed to fetch GitHub repository: {}", error))
    })?;

    GithubConnectionRepo::reconnect(
        &state.storage.db_pool,
        &app.id,
        payload.installation_id,
        payload.repository_id,
        &app.default_branch,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn disconnect_github(
    State(state): State<AppState>,
    ActiveAppOwner { app, user: _ }: ActiveAppOwner,
) -> HttpResult<impl IntoResponse> {
    if app.source != AppSource::Github {
        return Err(HttpError::bad_request("App does not use GitHub"));
    }

    GithubConnectionRepo::update_status(
        &state.storage.db_pool,
        &app.id,
        ConnectionStatus::Disconnected,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, Validate)]
struct UpdateBranchReq {
    #[serde(deserialize_with = "trim_string")]
    #[garde(custom(not_empty))]
    branch: String,
}

async fn update_connection_branch(
    State(state): State<AppState>,
    ActiveAppOwner { mut app, user: _ }: ActiveAppOwner,
    ValidatedJson(payload): ValidatedJson<UpdateBranchReq>,
) -> HttpResult<impl IntoResponse> {
    if !matches!(app.source, AppSource::Git | AppSource::Github) {
        return Err(HttpError::bad_request(
            "App does not use a remote connection",
        ));
    }

    let branch = payload.branch;

    AppRepo::update_default_branch(&state.storage.db_pool, &app.id, &branch).await?;
    app.default_branch = branch.clone();

    sync_external_app(
        state.github_client().await.as_ref(),
        &state.storage,
        &state.runtime,
        &mut app,
    )
    .await
    .map_err(|e| HttpError::bad_request(format!("Failed to sync with new branch: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

async fn sync_app(
    State(state): State<AppState>,
    ActiveAppOwner { mut app, user: _ }: ActiveAppOwner,
) -> HttpResult<impl IntoResponse> {
    if !matches!(app.source, AppSource::Git | AppSource::Github) {
        return Err(HttpError::bad_request(
            "App does not use a remote connection",
        ));
    }

    sync_external_app(
        state.github_client().await.as_ref(),
        &state.storage,
        &state.runtime,
        &mut app,
    )
    .await
    .map_err(|e| HttpError::bad_request(format!("Failed to sync repository: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}
