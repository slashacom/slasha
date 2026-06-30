use chrono::{DateTime, Datelike, Duration, NaiveDateTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;

const MONTHS: &[(&str, u32)] = &[
    ("jan", 1),
    ("feb", 2),
    ("mar", 3),
    ("apr", 4),
    ("may", 5),
    ("jun", 6),
    ("jul", 7),
    ("aug", 8),
    ("sep", 9),
    ("oct", 10),
    ("nov", 11),
    ("dec", 12),
];

const DOWS: &[(&str, u32)] = &[
    ("sun", 0),
    ("mon", 1),
    ("tue", 2),
    ("wed", 3),
    ("thu", 4),
    ("fri", 5),
    ("sat", 6),
];

// Search horizon for the next matching minute. Caps the worst case (e.g. an
// impossible date) at roughly five years so the loop always terminates.
const MAX_LOOKAHEAD_MINUTES: i64 = 366 * 24 * 60 * 5;

#[derive(Debug, Clone)]
pub struct CronSchedule {
    minutes: Vec<bool>,
    hours: Vec<bool>,
    doms: Vec<bool>,
    months: Vec<bool>,
    dows: Vec<bool>,
    dom_restricted: bool,
    dow_restricted: bool,
}

pub fn parse(expr: &str) -> Result<CronSchedule, String> {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return Err(format!(
            "expected 5 fields (minute hour day-of-month month day-of-week), got {}",
            fields.len()
        ));
    }

    let minutes = parse_field(fields[0], 0, 59, &[])?;
    let hours = parse_field(fields[1], 0, 23, &[])?;
    let doms = parse_field(fields[2], 1, 31, &[])?;
    let months = parse_field(fields[3], 1, 12, MONTHS)?;
    let dows_raw = parse_field(fields[4], 0, 7, DOWS)?;

    let mut dows = vec![false; 7];
    for (value, on) in dows_raw.iter().enumerate() {
        if *on {
            dows[value % 7] = true;
        }
    }

    Ok(CronSchedule {
        minutes,
        hours,
        doms,
        months,
        dows,
        dom_restricted: fields[2] != "*",
        dow_restricted: fields[4] != "*",
    })
}

pub fn parse_timezone(timezone: &str) -> Result<Tz, String> {
    timezone
        .parse::<Tz>()
        .map_err(|_| format!("invalid timezone '{}'", timezone))
}

impl CronSchedule {
    pub fn next_after(&self, after: DateTime<Utc>, tz: Tz) -> Option<DateTime<Utc>> {
        let local = after.with_timezone(&tz).naive_local();
        let mut candidate = local
            .with_second(0)
            .and_then(|dt| dt.with_nanosecond(0))
            .unwrap_or(local)
            + Duration::minutes(1);

        for _ in 0..MAX_LOOKAHEAD_MINUTES {
            if self.matches(&candidate) {
                match tz.from_local_datetime(&candidate) {
                    chrono::LocalResult::Single(dt) | chrono::LocalResult::Ambiguous(dt, _) => {
                        return Some(dt.with_timezone(&Utc));
                    }
                    chrono::LocalResult::None => {}
                }
            }
            candidate += Duration::minutes(1);
        }

        None
    }

    pub fn upcoming(&self, from: DateTime<Utc>, tz: Tz, count: usize) -> Vec<DateTime<Utc>> {
        let mut out = Vec::with_capacity(count);
        let mut cursor = from;
        for _ in 0..count {
            match self.next_after(cursor, tz) {
                Some(next) => {
                    out.push(next);
                    cursor = next;
                }
                None => break,
            }
        }
        out
    }

    fn matches(&self, dt: &NaiveDateTime) -> bool {
        if !self.minutes[dt.minute() as usize] {
            return false;
        }
        if !self.hours[dt.hour() as usize] {
            return false;
        }
        if !self.months[dt.month() as usize] {
            return false;
        }

        let dom_ok = self.doms[dt.day() as usize];
        let dow_ok = self.dows[dt.weekday().num_days_from_sunday() as usize];

        match (self.dom_restricted, self.dow_restricted) {
            (true, true) => dom_ok || dow_ok,
            (true, false) => dom_ok,
            (false, true) => dow_ok,
            (false, false) => true,
        }
    }
}

fn parse_field(spec: &str, min: u32, max: u32, names: &[(&str, u32)]) -> Result<Vec<bool>, String> {
    let mut set = vec![false; (max + 1) as usize];

    for term in spec.split(',') {
        let term = term.trim();
        if term.is_empty() {
            return Err(format!("empty term in '{}'", spec));
        }

        let (range_part, step) = match term.split_once('/') {
            Some((range, step)) => {
                let step: u32 = step
                    .parse()
                    .map_err(|_| format!("invalid step '{}' in '{}'", step, spec))?;
                if step == 0 {
                    return Err(format!("step cannot be zero in '{}'", spec));
                }
                (range, step)
            }
            None => (term, 1),
        };

        let (lo, hi) = if range_part == "*" {
            (min, max)
        } else if let Some((start, end)) = range_part.split_once('-') {
            (
                parse_value(start, names, min, max)?,
                parse_value(end, names, min, max)?,
            )
        } else {
            let value = parse_value(range_part, names, min, max)?;
            if step == 1 {
                (value, value)
            } else {
                (value, max)
            }
        };

        if lo > hi {
            return Err(format!(
                "range start {} is after end {} in '{}'",
                lo, hi, spec
            ));
        }

        let mut value = lo;
        while value <= hi {
            set[value as usize] = true;
            value += step;
        }
    }

    Ok(set)
}

fn parse_value(token: &str, names: &[(&str, u32)], min: u32, max: u32) -> Result<u32, String> {
    let token = token.trim();
    if let Ok(value) = token.parse::<u32>() {
        if value < min || value > max {
            return Err(format!("value {} out of range {}-{}", value, min, max));
        }
        return Ok(value);
    }

    let lower = token.to_ascii_lowercase();
    for (name, value) in names {
        if *name == lower {
            return Ok(*value);
        }
    }

    Err(format!("invalid value '{}'", token))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::UTC;

    fn at(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn rejects_wrong_field_count() {
        assert!(parse("* * * *").is_err());
        assert!(parse("* * * * * *").is_err());
    }

    #[test]
    fn step_expands_minutes() {
        let schedule = parse("*/15 * * * *").unwrap();
        let next = schedule
            .next_after(at("2026-01-01T00:02:00Z"), UTC)
            .unwrap();
        assert_eq!(next, at("2026-01-01T00:15:00Z"));
    }

    #[test]
    fn daily_midnight() {
        let schedule = parse("0 0 * * *").unwrap();
        let next = schedule
            .next_after(at("2026-01-01T08:00:00Z"), UTC)
            .unwrap();
        assert_eq!(next, at("2026-01-02T00:00:00Z"));
    }

    #[test]
    fn weekday_morning_skips_weekend() {
        // 2026-01-03 is a Saturday; next weekday 09:00 is Monday the 5th.
        let schedule = parse("0 9 * * 1-5").unwrap();
        let next = schedule
            .next_after(at("2026-01-03T12:00:00Z"), UTC)
            .unwrap();
        assert_eq!(next, at("2026-01-05T09:00:00Z"));
    }

    #[test]
    fn dom_and_dow_union() {
        // Runs on the 1st OR any Monday.
        let schedule = parse("0 0 1 * 1").unwrap();
        let next = schedule
            .next_after(at("2026-01-02T00:00:00Z"), UTC)
            .unwrap();
        assert_eq!(next, at("2026-01-05T00:00:00Z"));
    }

    #[test]
    fn named_fields() {
        let schedule = parse("0 0 1 jan mon").unwrap();
        assert!(
            schedule
                .next_after(at("2026-01-01T00:00:00Z"), UTC)
                .is_some()
        );
    }
}
