//! Local highscore tables, one per game mode
//! (boundary × field size × speed).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use snake_core::BoundaryMode;

use crate::settings::{Settings, Speed};

pub const MAX_ENTRIES: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub score: u32,
    /// ISO date (UTC), e.g. "2026-06-12".
    pub date: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Highscores {
    tables: BTreeMap<String, Vec<Entry>>,
}

impl Highscores {
    pub fn table(&self, key: &str) -> &[Entry] {
        self.tables.get(key).map_or(&[], Vec::as_slice)
    }

    /// Would this score make it into the top list of its mode?
    pub fn qualifies(&self, key: &str, score: u32) -> bool {
        if score == 0 {
            return false;
        }
        let table = self.table(key);
        table.len() < MAX_ENTRIES || table.last().is_some_and(|worst| score > worst.score)
    }

    pub fn insert(&mut self, key: &str, entry: Entry) {
        let table = self.tables.entry(key.to_owned()).or_default();
        table.push(entry);
        // Stable sort: on equal scores, earlier entries keep their rank.
        table.sort_by_key(|e| std::cmp::Reverse(e.score));
        table.truncate(MAX_ENTRIES);
    }
}

/// Stable identifier of a game mode, used as table key
/// (and later as API path segment for global leaderboards).
pub fn mode_key(settings: &Settings) -> String {
    let boundary = match settings.boundary {
        BoundaryMode::Walls => "walls",
        BoundaryMode::Periodic => "torus",
    };
    let (width, height) = settings.dimensions();
    let speed = match settings.speed {
        Speed::Slow => "slow",
        Speed::Normal => "normal",
        Speed::Fast => "fast",
    };
    format!("{boundary}-{width}x{height}-{speed}")
}

/// Current UTC date as "YYYY-MM-DD", without pulling in a date-time crate.
pub fn today_iso() -> String {
    let days = unix_days();
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(not(target_arch = "wasm32"))]
fn unix_days() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_secs() / 86_400) as i64)
        .unwrap_or(0)
}

#[cfg(target_arch = "wasm32")]
fn unix_days() -> i64 {
    (eframe::web_sys::js_sys::Date::now() / 86_400_000.0) as i64
}

/// Days since 1970-01-01 → (year, month, day).
/// Howard Hinnant's `civil_from_days` algorithm.
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

    fn entry(name: &str, score: u32) -> Entry {
        Entry {
            name: name.into(),
            score,
            date: "2026-06-12".into(),
        }
    }

    #[test]
    fn insert_sorts_descending_and_truncates() {
        let mut hs = Highscores::default();
        for i in 0..15 {
            hs.insert("walls-16x12-normal", entry(&format!("p{i}"), i));
        }
        let table = hs.table("walls-16x12-normal");
        assert_eq!(table.len(), MAX_ENTRIES);
        assert_eq!(table.first().unwrap().score, 14);
        assert_eq!(table.last().unwrap().score, 5);
        assert!(table.windows(2).all(|w| w[0].score >= w[1].score));
    }

    #[test]
    fn tables_are_separate_per_mode() {
        let mut hs = Highscores::default();
        hs.insert("walls-16x12-normal", entry("a", 3));
        hs.insert("torus-16x12-normal", entry("b", 7));
        assert_eq!(hs.table("walls-16x12-normal").len(), 1);
        assert_eq!(hs.table("torus-16x12-normal").len(), 1);
        assert!(hs.table("walls-24x18-fast").is_empty());
    }

    #[test]
    fn qualification_rules() {
        let mut hs = Highscores::default();
        assert!(!hs.qualifies("k", 0), "score 0 never qualifies");
        assert!(hs.qualifies("k", 1), "any positive score on empty table");
        for i in 10..20 {
            hs.insert("k", entry("x", i));
        }
        assert!(!hs.qualifies("k", 9), "below the worst of a full table");
        assert!(!hs.qualifies("k", 10), "equal to the worst of a full table");
        assert!(hs.qualifies("k", 11));
    }

    #[test]
    fn ties_keep_insertion_order() {
        let mut hs = Highscores::default();
        hs.insert("k", entry("first", 5));
        hs.insert("k", entry("second", 5));
        let table = hs.table("k");
        assert_eq!(table[0].name, "first");
        assert_eq!(table[1].name, "second");
    }

    #[test]
    fn civil_from_days_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1)); // leap year
        assert_eq!(civil_from_days(20_617), (2026, 6, 13));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }

    #[test]
    fn mode_key_distinguishes_all_dimensions() {
        use crate::settings::SizePreset;
        let mut s = Settings {
            preset: SizePreset::Small,
            ..Default::default()
        };
        let base = mode_key(&s);
        assert_eq!(base, "walls-16x12-normal");
        s.boundary = BoundaryMode::Periodic;
        assert_ne!(mode_key(&s), base);
        s.boundary = BoundaryMode::Walls;
        s.speed = Speed::Fast;
        assert_ne!(mode_key(&s), base);
        s.speed = Speed::Normal;
        s.preset = SizePreset::Large;
        assert_ne!(mode_key(&s), base);
    }
}
