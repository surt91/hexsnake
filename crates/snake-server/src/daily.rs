//! Daily challenge: a deterministic per-day seed and the current UTC date.
//!
//! The seed is derived purely from the day number and a server secret, so it
//! is stable for the whole day and identical across restarts. The client can
//! fall back to the same date-based seed when the server is unreachable (it
//! just cannot mix in the secret, which only matters for unpredictability).

use snake_core::BoundaryMode;

/// Board the daily challenge is played on (fixed: medium field, walls).
pub const DAILY_WIDTH: i32 = 24;
pub const DAILY_HEIGHT: i32 = 18;
pub const DAILY_BOUNDARY: BoundaryMode = BoundaryMode::Walls;

/// Days since the Unix epoch (UTC), wall-clock based.
pub fn unix_days() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_secs() / 86_400) as i64)
        .unwrap_or(0)
}

/// Current UTC date as "YYYY-MM-DD".
pub fn today_iso() -> String {
    iso_from_days(unix_days())
}

/// Format a day number (since the epoch) as an ISO date.
pub fn iso_from_days(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Deterministic 32-bit daily seed for a given day number and secret.
pub fn seed_for_day(days: i64, secret: u64) -> u64 {
    // splitmix64 finalizer on (day ^ secret) for a well-mixed value, then
    // reduced to 32 bits to match the game's compact seed space.
    let mut z = (days as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ secret;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    z & 0xFFFF_FFFF
}

/// Howard Hinnant's `civil_from_days`: day number → (year, month, day).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_dates_are_correct() {
        assert_eq!(iso_from_days(0), "1970-01-01");
        assert_eq!(iso_from_days(20_617), "2026-06-13");
    }

    #[test]
    fn seed_is_stable_and_32_bit() {
        let a = seed_for_day(20_617, 42);
        let b = seed_for_day(20_617, 42);
        assert_eq!(a, b, "same day + secret ⇒ same seed");
        assert!(a <= u64::from(u32::MAX));
        assert_ne!(a, seed_for_day(20_618, 42), "different days differ");
    }
}
