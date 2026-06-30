use std::sync::Arc;

use chrono::Utc;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use slasha_db::github_app_config::GithubAppConfig;

#[derive(Debug, thiserror::Error)]
pub enum GithubError {
    #[error("GitHub installation or repository is no longer accessible")]
    AccessRevoked,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<reqwest::Error> for GithubError {
    fn from(err: reqwest::Error) -> Self {
        GithubError::Other(anyhow::anyhow!(err))
    }
}

pub type GithubResult<T> = Result<T, GithubError>;

#[derive(Clone)]
pub struct GithubClient {
    inner: Arc<Inner>,
}

struct Inner {
    http: Client,
    app_id: String,
    client_id: String,
    client_secret: String,
    private_key: EncodingKey,
    webhook_secret: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GithubRepository {
    pub id: i64,
    pub full_name: String,
    pub html_url: String,
    pub clone_url: String,
    pub default_branch: String,
    pub private: bool,
}

#[derive(Serialize)]
struct AppClaims<'a> {
    iat: i64,
    exp: i64,
    iss: &'a str,
}

#[derive(Deserialize)]
struct AppInfo {
    html_url: String,
}

#[derive(Deserialize)]
struct OAuthToken {
    access_token: String,
}

#[derive(Deserialize)]
struct InstallationToken {
    token: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GithubInstallationInfo {
    pub id: i64,
    pub html_url: String,
}

#[derive(Deserialize)]
struct Repositories {
    repositories: Vec<GithubRepository>,
}

impl GithubClient {
    pub fn from_config(config: &GithubAppConfig) -> anyhow::Result<Self> {
        let private_key_pem = config.private_key.replace("\\n", "\n");
        let private_key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())?;
        Ok(Self {
            inner: Arc::new(Inner {
                http: Client::builder().user_agent("slasha").build()?,
                app_id: config.app_id.clone(),
                client_id: config.client_id.clone(),
                client_secret: config.client_secret.clone(),
                private_key,
                webhook_secret: config.webhook_secret.clone().into_bytes(),
            }),
        })
    }

    fn generate_app_jwt(&self) -> anyhow::Result<String> {
        let now = Utc::now().timestamp();
        Ok(encode(
            &Header::new(Algorithm::RS256),
            &AppClaims {
                iat: now - 60,
                exp: now + 9 * 60,
                iss: &self.inner.app_id,
            },
            &self.inner.private_key,
        )?)
    }

    fn build_api_url(&self, path: &str) -> String {
        format!("https://api.github.com{}", path)
    }

    pub fn webhook_secret(&self) -> &[u8] {
        &self.inner.webhook_secret
    }

    pub async fn get_installation_url(&self, state: &str) -> GithubResult<String> {
        let app = self
            .inner
            .http
            .get(self.build_api_url("/app"))
            .bearer_auth(self.generate_app_jwt()?)
            .send()
            .await?
            .error_for_status()?
            .json::<AppInfo>()
            .await?;
        Ok(format!(
            "{}/installations/new?state={}",
            app.html_url,
            urlencoding::encode(state)
        ))
    }

    pub async fn exchange_oauth_code(&self, code: &str) -> GithubResult<String> {
        let response = self
            .inner
            .http
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .json(&serde_json::json!({
                "client_id": self.inner.client_id,
                "client_secret": self.inner.client_secret,
                "code": code,
            }))
            .send()
            .await?
            .error_for_status()?
            .json::<OAuthToken>()
            .await?;
        Ok(response.access_token)
    }

    pub async fn user_has_installation_access(
        &self,
        user_token: &str,
        installation_id: i64,
    ) -> GithubResult<bool> {
        let response = self
            .inner
            .http
            .get(self.build_api_url(&format!(
                "/user/installations/{}/repositories?per_page=1",
                installation_id
            )))
            .bearer_auth(user_token)
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    pub async fn get_installation_token(&self, installation_id: i64) -> GithubResult<String> {
        let response = self
            .inner
            .http
            .post(self.build_api_url(&format!(
                "/app/installations/{}/access_tokens",
                installation_id
            )))
            .bearer_auth(self.generate_app_jwt()?)
            .send()
            .await?;

        if matches!(
            response.status(),
            StatusCode::NOT_FOUND | StatusCode::FORBIDDEN
        ) {
            return Err(GithubError::AccessRevoked);
        }

        Ok(response
            .error_for_status()?
            .json::<InstallationToken>()
            .await?
            .token)
    }

    pub async fn get_installation(
        &self,
        installation_id: i64,
    ) -> GithubResult<GithubInstallationInfo> {
        let response = self
            .inner
            .http
            .get(self.build_api_url(&format!("/app/installations/{}", installation_id)))
            .bearer_auth(self.generate_app_jwt()?)
            .send()
            .await?;
        if matches!(
            response.status(),
            StatusCode::NOT_FOUND | StatusCode::FORBIDDEN
        ) {
            return Err(GithubError::AccessRevoked);
        }
        Ok(response
            .error_for_status()?
            .json::<GithubInstallationInfo>()
            .await?)
    }

    pub async fn delete_installation(&self, installation_id: i64) -> GithubResult<()> {
        let response = self
            .inner
            .http
            .delete(self.build_api_url(&format!("/app/installations/{}", installation_id)))
            .bearer_auth(self.generate_app_jwt()?)
            .send()
            .await?;
        if matches!(
            response.status(),
            StatusCode::NOT_FOUND | StatusCode::FORBIDDEN
        ) {
            return Err(GithubError::AccessRevoked);
        }
        response.error_for_status()?;
        Ok(())
    }

    pub async fn get_repositories(
        &self,
        installation_id: i64,
    ) -> GithubResult<Vec<GithubRepository>> {
        let max_per_page = 100;
        let token = self.get_installation_token(installation_id).await?;

        let mut all_repositories = Vec::new();
        let mut page = 1u32;

        loop {
            let batch = self
                .inner
                .http
                .get(self.build_api_url(&format!(
                    "/installation/repositories?per_page={}&page={}",
                    max_per_page, page
                )))
                .bearer_auth(&token)
                .send()
                .await?
                .error_for_status()?
                .json::<Repositories>()
                .await?
                .repositories;

            let fetched = batch.len() as u32;
            all_repositories.extend(batch);

            if fetched < max_per_page {
                break;
            }
            page += 1;
        }

        Ok(all_repositories)
    }

    /// this is a client utility method that uses get_repository_with_token internally
    /// but discards the token
    pub async fn get_repository(
        &self,
        installation_id: i64,
        repository_id: i64,
    ) -> GithubResult<GithubRepository> {
        Ok(self
            .get_repository_with_token(installation_id, repository_id)
            .await?
            .0)
    }

    pub async fn get_repository_with_token(
        &self,
        installation_id: i64,
        repository_id: i64,
    ) -> GithubResult<(GithubRepository, String)> {
        let token = self.get_installation_token(installation_id).await?;
        let repository = self
            .inner
            .http
            .get(self.build_api_url(&format!("/repositories/{}", repository_id)))
            .bearer_auth(&token)
            .send()
            .await?;
        if matches!(
            repository.status(),
            StatusCode::NOT_FOUND | StatusCode::FORBIDDEN
        ) {
            return Err(GithubError::AccessRevoked);
        }
        let repository = repository
            .error_for_status()?
            .json::<GithubRepository>()
            .await?;

        if repository.id != repository_id {
            return Err(GithubError::Other(anyhow::anyhow!(
                "GitHub returned an unexpected repository"
            )));
        }
        Ok((repository, token))
    }
}
