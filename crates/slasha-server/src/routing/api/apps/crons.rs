use std::{str::FromStr, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State},
    response::{
        IntoResponse,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
};
use bollard::Docker;
use chrono::{NaiveDateTime, Utc};
use futures_util::{StreamExt, stream};
use serde::Deserialize;
use slasha_db::{
    DbPool,
    cron::{CronJob, CronRunTrigger, CronRuntime},
    repos::cron::{CronJobRepo, CronRunRepo, new_run},
};
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use crate::{
    cron::{runner, schedule},
    docker::logs::{LogKey, LogManager},
    error::{HttpError, HttpResult},
    extractors::app::ActiveApp,
    state::AppState,
};

const DEFAULT_TIMEOUT_SECS: i32 = 3600;
const RUN_HISTORY_LIMIT: i64 = 50;
const PREVIEW_COUNT: usize = 5;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_crons).post(create_cron))
        .route("/preview", post(preview_schedule))
        .route(
            "/{cron_id}",
            get(get_cron).put(update_cron).delete(delete_cron),
        )
        .route("/{cron_id}/run", post(run_now))
        .route("/{cron_id}/runs", get(list_runs))
        .route("/{cron_id}/runs/{run_id}/logs", get(stream_run_logs))
}

#[derive(Deserialize)]
struct CronInput {
    name: String,
    schedule: String,
    command: String,
    timezone: Option<String>,
    enabled: bool,
    timeout_secs: Option<i32>,
    runtime: Option<String>,
}

#[derive(Deserialize)]
struct PreviewInput {
    schedule: String,
    timezone: Option<String>,
}

struct ValidatedCron {
    name: String,
    schedule: String,
    command: String,
    timezone: String,
    enabled: bool,
    timeout_secs: i32,
    runtime: CronRuntime,
    next_run_at: Option<NaiveDateTime>,
}

fn validate(input: CronInput) -> HttpResult<ValidatedCron> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(HttpError::bad_request("Name is required."));
    }

    let command = input.command.trim().to_string();
    if command.is_empty() {
        return Err(HttpError::bad_request("Command is required."));
    }

    let schedule_expr = input.schedule.trim().to_string();
    let parsed = schedule::parse(&schedule_expr).map_err(HttpError::bad_request)?;

    let timezone = input
        .timezone
        .map(|tz| tz.trim().to_string())
        .filter(|tz| !tz.is_empty())
        .unwrap_or_else(|| "UTC".to_string());
    let tz = schedule::parse_timezone(&timezone).map_err(HttpError::bad_request)?;

    let timeout_secs = input.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);
    if timeout_secs <= 0 {
        return Err(HttpError::bad_request("Timeout must be greater than zero."));
    }

    let runtime = match input.runtime {
        Some(runtime) if !runtime.trim().is_empty() => CronRuntime::from_str(runtime.trim())
            .map_err(|_| HttpError::bad_request("Invalid runtime."))?,
        _ => CronRuntime::App,
    };

    let next_run_at = if input.enabled {
        parsed.next_after(Utc::now(), tz).map(|dt| dt.naive_utc())
    } else {
        None
    };

    Ok(ValidatedCron {
        name,
        schedule: schedule_expr,
        command,
        timezone,
        enabled: input.enabled,
        timeout_secs,
        runtime,
        next_run_at,
    })
}

async fn list_crons(
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
) -> HttpResult<impl IntoResponse> {
    let crons = CronJobRepo::list_for_app(&db_pool, &app.id).await?;

    let mut items = Vec::with_capacity(crons.len());
    for job in crons {
        let last_run = CronRunRepo::latest_for_job(&db_pool, &job.id).await?;
        let mut value = serde_json::to_value(&job).map_err(HttpError::internal)?;
        if let serde_json::Value::Object(map) = &mut value {
            map.insert(
                "last_run".to_string(),
                serde_json::to_value(last_run).map_err(HttpError::internal)?,
            );
        }
        items.push(value);
    }

    Ok(Json(serde_json::json!({ "crons": items })))
}

async fn get_cron(
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, cron_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let cron = CronJobRepo::find(&db_pool, &cron_id, &app.id).await?;
    Ok(Json(serde_json::json!({ "cron": cron })))
}

async fn create_cron(
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
    Json(input): Json<CronInput>,
) -> HttpResult<impl IntoResponse> {
    let valid = validate(input)?;
    let now = Utc::now().naive_utc();

    let cron = CronJob {
        id: Uuid::new_v4().to_string(),
        app_id: app.id.clone(),
        name: valid.name,
        schedule: valid.schedule,
        command: valid.command,
        timezone: valid.timezone,
        enabled: valid.enabled,
        timeout_secs: valid.timeout_secs,
        runtime: valid.runtime,
        last_run_at: None,
        next_run_at: valid.next_run_at,
        created_at: now,
        updated_at: now,
    };

    let cron = CronJobRepo::create(&db_pool, cron).await?;
    Ok(Json(serde_json::json!({ "cron": cron })))
}

async fn update_cron(
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, cron_id)): Path<(String, String)>,
    Json(input): Json<CronInput>,
) -> HttpResult<impl IntoResponse> {
    let existing = CronJobRepo::find(&db_pool, &cron_id, &app.id).await?;
    let valid = validate(input)?;
    let now = Utc::now().naive_utc();

    let cron = CronJob {
        id: existing.id,
        app_id: app.id.clone(),
        name: valid.name,
        schedule: valid.schedule,
        command: valid.command,
        timezone: valid.timezone,
        enabled: valid.enabled,
        timeout_secs: valid.timeout_secs,
        runtime: valid.runtime,
        last_run_at: existing.last_run_at,
        next_run_at: valid.next_run_at,
        created_at: existing.created_at,
        updated_at: now,
    };

    let cron = CronJobRepo::update(&db_pool, &cron_id, cron).await?;
    Ok(Json(serde_json::json!({ "cron": cron })))
}

async fn delete_cron(
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, cron_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    CronJobRepo::find(&db_pool, &cron_id, &app.id).await?;
    CronJobRepo::delete(&db_pool, &cron_id, &app.id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn run_now(
    State(docker): State<Docker>,
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, cron_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    let job = CronJobRepo::find(&db_pool, &cron_id, &app.id).await?;

    if CronRunRepo::has_active(&db_pool, &job.id).await? {
        return Err(HttpError::bad_request(
            "A run is already in progress for this cron job.",
        ));
    }

    let run = CronRunRepo::create(&db_pool, new_run(&job.id, CronRunTrigger::Manual)).await?;

    let dispatched = run.clone();
    tokio::spawn(async move {
        runner::run_cron_job(db_pool, docker, log_manager, job, dispatched).await;
    });

    Ok(Json(serde_json::json!({ "run": run })))
}

async fn list_runs(
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, cron_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    CronJobRepo::find(&db_pool, &cron_id, &app.id).await?;
    let runs = CronRunRepo::list_for_job(&db_pool, &cron_id, RUN_HISTORY_LIMIT).await?;
    Ok(Json(serde_json::json!({ "runs": runs })))
}

async fn preview_schedule(
    ActiveApp { .. }: ActiveApp,
    Json(input): Json<PreviewInput>,
) -> HttpResult<impl IntoResponse> {
    let parsed = schedule::parse(input.schedule.trim()).map_err(HttpError::bad_request)?;
    let timezone = input
        .timezone
        .map(|tz| tz.trim().to_string())
        .filter(|tz| !tz.is_empty())
        .unwrap_or_else(|| "UTC".to_string());
    let tz = schedule::parse_timezone(&timezone).map_err(HttpError::bad_request)?;

    let next_runs: Vec<String> = parsed
        .upcoming(Utc::now(), tz, PREVIEW_COUNT)
        .into_iter()
        .map(|dt| dt.to_rfc3339())
        .collect();

    Ok(Json(serde_json::json!({ "next_runs": next_runs })))
}

async fn stream_run_logs(
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, cron_id, run_id)): Path<(String, String, String)>,
) -> HttpResult<
    Sse<impl futures_util::Stream<Item = std::result::Result<Event, std::convert::Infallible>>>,
> {
    CronJobRepo::find(&db_pool, &cron_id, &app.id).await?;
    CronRunRepo::find(&db_pool, &run_id, &cron_id).await?;

    let log = log_manager
        .get_logger(&LogKey::Cron {
            app_slug: app.slug.clone(),
            cron_run_id: run_id,
        })
        .await
        .map_err(HttpError::internal)?;

    let historical = log.get_historical().await?;
    let historical_stream = stream::iter(
        historical
            .into_iter()
            .map(|msg| Ok(Event::default().data(msg))),
    );

    let rx = log.subscribe();
    let live_stream = BroadcastStream::new(rx).map(|res| match res {
        Ok(msg) => Ok(Event::default().data(msg)),
        Err(e) => Ok(Event::default().event("error").data(e.to_string())),
    });

    let done_marker = stream::once(async { Ok(Event::default().data("[done]")) });
    let combined = historical_stream.chain(done_marker).chain(live_stream);

    Ok(Sse::new(combined).keep_alive(KeepAlive::default()))
}
