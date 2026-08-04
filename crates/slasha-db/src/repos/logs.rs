use chrono::NaiveDateTime;
use duckdb::params;
use tokio::sync::mpsc;

use crate::{
    connection::DuckdbPool,
    error::DbResult,
    models::logs::{LogPrefix, LogRecord, LogStream, ResourceKind},
};

pub struct LogsRepo;

pub enum FetchMode {
    Buffer(i64),
    Stream(mpsc::Sender<LogRecord>),
}

pub struct LogHistoryQuery {
    pub resource_id: String,
    pub mode: FetchMode,
    pub before_ts: Option<NaiveDateTime>,
    pub after_ts: Option<NaiveDateTime>,
    pub search: Option<String>,
    pub stream_filter: Option<LogStream>,
    pub prefix_filter: Option<String>,
    pub resource_kind_filter: Option<ResourceKind>,
}

impl LogsRepo {
    pub async fn insert_batch(pool: &DuckdbPool, records: Vec<LogRecord>) -> DbResult<()> {
        if records.is_empty() {
            return Ok(());
        }

        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            conn.execute_batch("BEGIN")?;

            for record in &records {
                conn.execute(
                    "INSERT INTO logs (id, timestamp, resource_kind, resource_id, app_id, prefix, stream, message) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    params![
                        record.id,
                        record.timestamp,
                        record.resource_kind.to_string(),
                        record.resource_id,
                        record.app_id,
                        record.prefix.as_ref().map(|p| p.to_string()),
                        record.stream.to_string(),
                        record.message,
                    ],
                )?;
            }

            conn.execute_batch("COMMIT")?;
            Ok(())
        })
        .await?
    }

    pub async fn get_history(
        pool: &DuckdbPool,
        query: LogHistoryQuery,
    ) -> DbResult<Vec<LogRecord>> {
        let pool = pool.clone();
        let resource_id = query.resource_id;
        let search = query.search.map(|s| format!("%{}%", s));
        let stream_str = query.stream_filter.map(|s| s.to_string());
        let prefix_str = query.prefix_filter.map(|p| p.to_string());
        let kind_str = query.resource_kind_filter.map(|k| k.to_string());
        let before_ts = query.before_ts;
        let after_ts = query.after_ts;
        let mode = query.mode;

        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;

            let (where_clause, mut params) = build_query_params(
                resource_id,
                kind_str,
                search,
                stream_str,
                prefix_str,
                before_ts,
                after_ts,
            );

            let sql = match mode {
                FetchMode::Buffer(limit) => {
                    params.push(Box::new(limit));
                    format!(
                        "SELECT id, timestamp, resource_kind, resource_id, app_id, prefix, stream, message \
                         FROM logs WHERE {} ORDER BY timestamp DESC LIMIT ?",
                        where_clause
                    )
                }
                FetchMode::Stream(_) => {
                    format!(
                        "SELECT id, timestamp, resource_kind, resource_id, app_id, prefix, stream, message \
                         FROM logs WHERE {} ORDER BY timestamp ASC",
                        where_clause
                    )
                }
            };

            let mut stmt = conn.prepare(&sql)?;
            let param_refs: Vec<&dyn duckdb::ToSql> = params.iter().map(|p| p.as_ref()).collect();

            match mode {
                FetchMode::Buffer { .. } => {
                    let rows = stmt.query_map(duckdb::params_from_iter(param_refs), map_row)?;
                    let mut records = rows.collect::<Result<Vec<_>, _>>()?;
                    records.reverse();
                    Ok(records)
                }
                FetchMode::Stream(tx) => {
                    let mut rows = stmt.query(duckdb::params_from_iter(param_refs))?;
                    while let Some(row) = rows.next()? {
                        let record = map_row(row)?;
                        if tx.blocking_send(record).is_err() {
                            break;
                        }
                    }
                    Ok(vec![])
                }
            }
        })
        .await?
    }

    pub async fn delete_by_resource_id(pool: &DuckdbPool, resource_id: &str) -> DbResult<usize> {
        let pool = pool.clone();
        let resource_id = resource_id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let count = conn.execute(
                "DELETE FROM logs WHERE resource_id = ?",
                params![resource_id],
            )?;
            Ok(count)
        })
        .await?
    }

    pub async fn delete_by_app_id(pool: &DuckdbPool, app_id: &str) -> DbResult<usize> {
        let pool = pool.clone();
        let app_id = app_id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let count = conn.execute("DELETE FROM logs WHERE app_id = ?", params![app_id])?;
            Ok(count)
        })
        .await?
    }
}

fn build_query_params(
    resource_id: String,
    kind_str: Option<String>,
    search: Option<String>,
    stream_str: Option<String>,
    prefix_str: Option<String>,
    before_ts: Option<NaiveDateTime>,
    after_ts: Option<NaiveDateTime>,
) -> (String, Vec<Box<dyn duckdb::ToSql>>) {
    let mut conditions = vec!["resource_id = ?"];
    let mut params: Vec<Box<dyn duckdb::ToSql>> = vec![Box::new(resource_id)];

    if let Some(k) = kind_str {
        conditions.push("resource_kind = ?");
        params.push(Box::new(k));
    }
    if let Some(s) = search {
        conditions.push("message ILIKE ?");
        params.push(Box::new(s));
    }
    if let Some(st) = stream_str {
        conditions.push("stream = ?");
        params.push(Box::new(st));
    }
    if let Some(p) = prefix_str {
        conditions.push("prefix ILIKE ?");
        params.push(Box::new(format!("{}%", p)));
    }
    if let Some(ts) = before_ts {
        conditions.push("timestamp < ?");
        params.push(Box::new(ts));
    }
    if let Some(ts) = after_ts {
        conditions.push("timestamp > ?");
        params.push(Box::new(ts));
    }

    (conditions.join(" AND "), params)
}

fn map_row(row: &duckdb::Row<'_>) -> duckdb::Result<LogRecord> {
    let resource_kind_str: String = row.get(2)?;
    let prefix_str: Option<String> = row.get(5)?;
    let stream_str: String = row.get(6)?;

    Ok(LogRecord {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        resource_kind: resource_kind_str
            .parse::<ResourceKind>()
            .unwrap_or(ResourceKind::Deployment),
        resource_id: row.get(3)?,
        app_id: row.get(4)?,
        prefix: prefix_str.and_then(|s| s.parse::<LogPrefix>().ok()),
        stream: stream_str.parse::<LogStream>().unwrap_or(LogStream::Stdout),
        message: row.get(7)?,
    })
}
