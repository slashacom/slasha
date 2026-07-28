pub mod scheduler;

use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Tz;
pub use scheduler::spawn_cron_scheduler;

/// Computes the next scheduled run timestamp for a cron job, returned as NaiveDateTime in UTC.
///
/// # Arguments
///
/// * `schedule` - Cron expression string.
/// * `timezone` - Timezone identifier string.
/// * `from` - Reference datetime to calculate next run after.
///
/// # Returns
///
/// An [`anyhow::Result`] containing an optional [`NaiveDateTime`].
pub fn next_run_at(
    schedule: &str,
    timezone: &str,
    from: &DateTime<Utc>,
) -> anyhow::Result<Option<NaiveDateTime>> {
    let tz: Tz = timezone
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid timezone '{timezone}'"))?;

    match cron_parser::parse(schedule, &from.with_timezone(&tz)) {
        Ok(next) => Ok(Some(next.with_timezone(&Utc).naive_utc())),
        Err(e) => Err(anyhow::anyhow!("invalid cron schedule: {e}")),
    }
}

/// Generates upcoming execution timestamps for schedule preview endpoints.
///
/// # Arguments
///
/// * `schedule` - Cron expression string.
/// * `timezone` - Timezone identifier string.
/// * `from` - Reference datetime to start preview from.
/// * `count` - Maximum number of upcoming execution timestamps to calculate.
///
/// # Returns
///
/// An [`anyhow::Result`] containing a vector of [`DateTime<Utc>`] timestamps.
pub fn upcoming_runs(
    schedule: &str,
    timezone: &str,
    from: &DateTime<Utc>,
    count: usize,
) -> anyhow::Result<Vec<DateTime<Utc>>> {
    let tz: Tz = timezone
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid timezone '{timezone}'"))?;

    let mut cursor = from.with_timezone(&tz);
    cron_parser::parse(schedule, &cursor)
        .map_err(|e| anyhow::anyhow!("invalid cron schedule: {e}"))?;

    let mut next_runs = Vec::with_capacity(count);
    for _ in 0..count {
        match cron_parser::parse(schedule, &cursor) {
            Ok(next) => {
                next_runs.push(next.with_timezone(&Utc));
                cursor = next;
            }
            Err(_) => break,
        }
    }

    Ok(next_runs)
}
