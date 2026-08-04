use axum::{
    Json,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use bytes::Bytes;
use chrono::NaiveDateTime;
use futures_util::StreamExt;
use serde::Deserialize;
use slasha_db::{
    DuckdbPool,
    models::logs::{LogStream, ResourceKind},
    repos::logs::{FetchMode, LogHistoryQuery, LogsRepo},
};
use tokio_stream::wrappers::{BroadcastStream, ReceiverStream};

use crate::{HttpResult, logs::LogBus};

/// Query parameters for querying resource execution and runtime logs.
#[derive(Deserialize, Default)]
pub struct LogQuery {
    pub limit: Option<usize>,
    pub before_ts: Option<NaiveDateTime>,
    pub after_ts: Option<NaiveDateTime>,
    pub search: Option<String>,
    pub stream: Option<LogStream>,
    // prefix is kept as a string instead of the strict log prefix enum
    // to allow the db to use a like clause for partial matching (e.g. "dep" matches "deploy")
    pub prefix: Option<String>,
    pub resource_kind: Option<ResourceKind>,
    pub download: Option<bool>,
}

/// Helper for retrieving historical log records for any resource ID from DuckDB.
///
/// # Arguments
///
/// * `duckdb_pool` - Shared DuckDB connection pool ([`DuckdbPool`]).
/// * `resource_id` - Target resource ID string.
/// * `query` - Applied log filtering options ([`LogQuery`]).
///
/// # Returns
///
/// An [`HttpResult`] containing the JSON log record array response.
pub async fn fetch_resource_logs(
    duckdb_pool: &DuckdbPool,
    resource_id: &str,
    query: LogQuery,
) -> HttpResult<Response> {
    if query.download.unwrap_or(false) {
        return download_resource_logs(duckdb_pool, resource_id, query).await;
    }

    let history_query = LogHistoryQuery {
        resource_id: resource_id.to_string(),
        mode: FetchMode::Buffer(query.limit.unwrap_or(1000) as i64),
        before_ts: query.before_ts,
        after_ts: query.after_ts,
        search: query.search,
        stream_filter: query.stream,
        prefix_filter: query.prefix,
        resource_kind_filter: query.resource_kind,
    };

    let logs = LogsRepo::get_history(duckdb_pool, history_query).await?;

    Ok(Json(serde_json::json!({ "logs": logs })).into_response())
}

async fn download_resource_logs(
    duckdb_pool: &DuckdbPool,
    resource_id: &str,
    query: LogQuery,
) -> HttpResult<Response> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);

    tokio::spawn({
        let search = query.search;
        let stream = query.stream;
        let prefix = query.prefix;
        let kind = query.resource_kind;
        let resource_id = resource_id.to_string();
        let pool = duckdb_pool.clone();

        async move {
            let history_query = LogHistoryQuery {
                resource_id,
                mode: FetchMode::Stream(tx),
                before_ts: None,
                after_ts: None,
                search,
                stream_filter: stream,
                prefix_filter: prefix,
                resource_kind_filter: kind,
            };
            let _ = LogsRepo::get_history(&pool, history_query).await;
        }
    });

    let stream = ReceiverStream::new(rx).map(|record| {
        let prefix_str = match &record.prefix {
            Some(p) => format!(" [{}]", p),
            None => String::new(),
        };
        let line = format!(
            "[{}]{} [{}] {}\n",
            record.timestamp.format("%H:%M:%S%.3f"),
            prefix_str,
            record.stream,
            record.message
        );
        Ok::<_, axum::Error>(Bytes::from(line))
    });

    let body = axum::body::Body::from_stream(stream);
    let mut response = body.into_response();
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/plain; charset=utf-8"),
    );

    let content_disposition = format!("attachment; filename=\"logs-{}.txt\"", resource_id);
    if let Ok(header_value) = axum::http::HeaderValue::from_str(&content_disposition) {
        response
            .headers_mut()
            .insert(axum::http::header::CONTENT_DISPOSITION, header_value);
    }

    Ok(response)
}

/// Helper for streaming live SSE log events for any resource ID.
///
/// # Arguments
///
/// * `log_bus` - Runtime log broadcast bus ([`LogBus`]).
/// * `resource_id` - Target resource ID string.
///
/// # Returns
///
/// An [`HttpResult`] wrapping an SSE event stream.
pub async fn stream_resource_logs(log_bus: &LogBus, resource_id: &str) -> HttpResult<Response> {
    let rx = log_bus.subscribe(resource_id);
    let live_stream = BroadcastStream::new(rx).filter_map(|res| async move {
        match res {
            Ok(rec) => Event::default()
                .json_data(rec)
                .ok()
                .map(Ok::<Event, std::convert::Infallible>),
            Err(_) => None,
        }
    });

    Ok(Sse::new(live_stream)
        .keep_alive(KeepAlive::default())
        .into_response())
}
