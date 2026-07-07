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
use garde::Validate;
use serde::Deserialize;
use slasha_db::{
    DbPool,
    cron::{CronJobChangeset, CronRunStatus, CronRunTrigger, CronRuntime, NewCronJob, NewCronRun},
    repos::cron::{CronJobRepo, CronRunRepo},
};
use tokio_stream::wrappers::BroadcastStream;

use crate::{
    HttpError, HttpResult,
    cron::{runner, schedule},
    docker::logs::{LogKey, LogManager},
    extractors::{ValidatedJson, app::ActiveApp},
    routing::api::{
        deserialize::{trim_optional_string, trim_string},
        validation::not_empty,
    },
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

#[derive(Deserialize, Validate)]
struct CronInput {
    #[serde(deserialize_with = "trim_string")]
    #[garde(custom(not_empty))]
    name: String,
    #[serde(deserialize_with = "trim_string")]
    #[garde(custom(not_empty))]
    schedule: String,
    #[serde(deserialize_with = "trim_string")]
    #[garde(custom(not_empty))]
    command: String,
    #[serde(default, deserialize_with = "trim_optional_string")]
    #[garde(skip)]
    timezone: Option<String>,
    #[garde(skip)]
    enabled: bool,
    #[garde(inner(range(min = 1)))]
    timeout_secs: Option<i32>,
    #[serde(default, deserialize_with = "trim_optional_string")]
    #[garde(skip)]
    runtime: Option<String>,
}

#[derive(Deserialize, Validate)]
struct PreviewInput {
    #[serde(deserialize_with = "trim_string")]
    #[garde(custom(not_empty))]
    schedule: String,
    #[serde(default, deserialize_with = "trim_optional_string")]
    #[garde(skip)]
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
    let parsed = schedule::parse(&input.schedule).map_err(HttpError::bad_request)?;

    let timezone = input
        .timezone
        .map(|tz| tz.trim().to_string())
        .filter(|tz| !tz.is_empty())
        .unwrap_or_else(|| "UTC".to_string());
    let tz = schedule::parse_timezone(&timezone).map_err(HttpError::bad_request)?;

    let timeout_secs = input.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);

    let runtime = match input.runtime {
        Some(runtime) if !runtime.is_empty() => CronRuntime::from_str(&runtime)
            .map_err(|_| HttpError::bad_request("Invalid runtime."))?,
        _ => CronRuntime::App,
    };

    let next_run_at = if input.enabled {
        parsed.next_after(Utc::now(), tz).map(|dt| dt.naive_utc())
    } else {
        None
    };

    Ok(ValidatedCron {
        name: input.name,
        schedule: input.schedule,
        command: input.command,
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
    ValidatedJson(input): ValidatedJson<CronInput>,
) -> HttpResult<impl IntoResponse> {
    let valid = validate(input)?;

    let cron = NewCronJob {
        app_id: app.id.clone(),
        name: valid.name,
        schedule: valid.schedule,
        command: valid.command,
        timezone: valid.timezone,
        enabled: valid.enabled,
        timeout_secs: valid.timeout_secs,
        runtime: valid.runtime,
    };

    let cron = CronJobRepo::create(&db_pool, cron).await?;
    Ok(Json(serde_json::json!({ "cron": cron })))
}

async fn update_cron(
    State(db_pool): State<DbPool>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, cron_id)): Path<(String, String)>,
    ValidatedJson(input): ValidatedJson<CronInput>,
) -> HttpResult<impl IntoResponse> {
    let _existing = CronJobRepo::find(&db_pool, &cron_id, &app.id).await?;
    let valid = validate(input)?;
    let now = Utc::now().naive_utc();

    let cron = CronJobChangeset {
        name: valid.name,
        schedule: valid.schedule,
        command: valid.command,
        timezone: valid.timezone,
        enabled: valid.enabled,
        timeout_secs: valid.timeout_secs,
        runtime: valid.runtime,
        next_run_at: valid.next_run_at,
        updated_at: now,
    };

    let cron = CronJobRepo::update(&db_pool, &cron_id, cron).await?;
    Ok(Json(serde_json::json!({ "cron": cron })))
}

async fn delete_cron(
    State(db_pool): State<DbPool>,
    State(log_manager): State<Arc<LogManager>>,
    ActiveApp { app, .. }: ActiveApp,
    Path((_, cron_id)): Path<(String, String)>,
) -> HttpResult<impl IntoResponse> {
    CronJobRepo::find(&db_pool, &cron_id, &app.id).await?;

    // Capture run ids before deletion cascades the rows away; the FK cascade
    // clears the database rows but not their on-disk logs.
    let run_ids = CronRunRepo::list_ids_for_job(&db_pool, &cron_id).await?;
    CronJobRepo::delete(&db_pool, &cron_id, &app.id).await?;

    for run_id in run_ids {
        if let Err(err) = log_manager.delete_cron_run_logs(&app.slug, &run_id).await {
            tracing::warn!(target: "slasha::cron", run = %run_id, error = ?err, "failed to delete cron run logs");
        }
    }

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

    let new_run_data = NewCronRun {
        cron_job_id: job.id.clone(),
        status: CronRunStatus::Pending,
        trigger_kind: CronRunTrigger::Manual,
    };
    let run = CronRunRepo::create(&db_pool, new_run_data).await?;

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
    ValidatedJson(input): ValidatedJson<PreviewInput>,
) -> HttpResult<impl IntoResponse> {
    let parsed = schedule::parse(&input.schedule).map_err(HttpError::bad_request)?;
    let timezone = input
        .timezone
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
