//! Cron math in the user's time zone. Pure functions, no state.
//!
//! Decision (architecture): croner + chrono-tz do the parsing and DST rules; we only pick the zone.

use chrono::{TimeZone, Utc};
use chrono_tz::Tz;
use croner::Cron;
use std::str::FromStr;

/// The setting value that follows the operating system's zone.
pub const SYSTEM: &str = "system";

/// The OS zone, or UTC when the OS gives a name we do not know.
pub fn system_zone() -> Tz {
    iana_time_zone::get_timezone()
        .ok()
        .and_then(|n| n.parse::<Tz>().ok())
        .unwrap_or(Tz::UTC)
}

/// `system` or an IANA name such as `Asia/Kolkata`.
pub fn resolve_zone(setting: &str) -> Result<Tz, String> {
    if setting == SYSTEM {
        return Ok(system_zone());
    }
    setting
        .parse::<Tz>()
        .map_err(|_| format!("'{setting}' is not a time zone name, for example Europe/Berlin."))
}

pub fn validate(expr: &str) -> Result<(), String> {
    Cron::from_str(expr)
        .map(|_| ())
        .map_err(|e| format!("That schedule is not valid: {e}"))
}

/// The first run strictly after `after_ms` (epoch milliseconds), or None if the schedule never fires again.
pub fn next_run_ms(expr: &str, zone: Tz, after_ms: i64) -> Result<Option<i64>, String> {
    let cron = Cron::from_str(expr).map_err(|e| format!("That schedule is not valid: {e}"))?;
    let after = Utc
        .timestamp_millis_opt(after_ms)
        .single()
        .ok_or("That time is out of range.")?
        .with_timezone(&zone);
    Ok(cron
        .find_next_occurrence(&after, false)
        .ok()
        .map(|t| t.timestamp_millis()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(zone: Tz, y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
        zone.with_ymd_and_hms(y, mo, d, h, mi, 0)
            .earliest()
            .unwrap()
            .timestamp_millis()
    }

    #[test]
    fn midnight_follows_the_chosen_zone() {
        let after = ms(Tz::UTC, 2026, 5, 1, 12, 0);
        let utc = next_run_ms("0 0 * * *", Tz::UTC, after).unwrap().unwrap();
        let ist = next_run_ms("0 0 * * *", Tz::Asia__Kolkata, after)
            .unwrap()
            .unwrap();
        assert_eq!(utc, ms(Tz::UTC, 2026, 5, 2, 0, 0));
        assert_eq!(ist, ms(Tz::Asia__Kolkata, 2026, 5, 2, 0, 0));
        assert_ne!(utc, ist);
    }

    #[test]
    fn a_time_skipped_by_spring_forward_still_fires_that_day() {
        // US clocks jump 02:00 to 03:00 on 2026-03-08, so 02:30 does not exist.
        let zone = Tz::America__New_York;
        let after = ms(zone, 2026, 3, 8, 0, 0);
        let next = next_run_ms("30 2 * * *", zone, after).unwrap().unwrap();
        let day_end = ms(zone, 2026, 3, 9, 0, 0);
        assert!(next > after && next < day_end, "{next}");
    }

    #[test]
    fn a_repeated_hour_fires_once() {
        // US clocks repeat 01:00 to 02:00 on 2026-11-01.
        let zone = Tz::America__New_York;
        let after = ms(zone, 2026, 11, 1, 0, 0);
        let first = next_run_ms("30 1 * * *", zone, after).unwrap().unwrap();
        let second = next_run_ms("30 1 * * *", zone, first).unwrap().unwrap();
        assert!(second - first > 20 * 3_600_000, "ran twice in one night");
    }

    #[test]
    fn bad_input_is_a_readable_error() {
        assert!(validate("not a cron").is_err());
        assert!(resolve_zone("Mars/Olympus").is_err());
        assert!(resolve_zone("Asia/Kolkata").is_ok());
        assert!(validate("0 0 * * *").is_ok());
    }
}
