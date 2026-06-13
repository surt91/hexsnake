//! SQLite-backed highscore storage.
//!
//! One table holds all entries, keyed by mode string (preset modes and the
//! daily-challenge modes `"daily-YYYY-MM-DD"` share the schema). Submissions
//! are pruned to the top `max_entries` per mode so storage stays bounded.

use std::path::Path;

use rusqlite::Connection;
use snake_core::api::ScoreEntry;

pub struct Db {
    conn: Connection,
}

impl Db {
    /// Open (and migrate) a database at `path`, or in memory if `None`.
    pub fn open(path: Option<&Path>) -> rusqlite::Result<Self> {
        let conn = match path {
            Some(p) => Connection::open(p)?,
            None => Connection::open_in_memory()?,
        };
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        conn.pragma_update(None, "foreign_keys", "ON").ok();
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS scores (
                 id         INTEGER PRIMARY KEY,
                 mode       TEXT    NOT NULL,
                 name       TEXT    NOT NULL,
                 score      INTEGER NOT NULL,
                 date       TEXT    NOT NULL,
                 created_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_scores_mode_score
                 ON scores(mode, score DESC, id ASC);",
        )?;
        Ok(())
    }

    /// Top `limit` entries for a mode, highest score first; ties broken by
    /// insertion order (older first).
    pub fn top(&self, mode: &str, limit: usize) -> rusqlite::Result<Vec<ScoreEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, score, date FROM scores
             WHERE mode = ?1 ORDER BY score DESC, id ASC LIMIT ?2",
        )?;
        let rows = stmt.query_map((mode, limit as i64), |row| {
            Ok(ScoreEntry {
                name: row.get(0)?,
                score: row.get::<_, i64>(1)? as u32,
                date: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    /// Would this score make it into the top `limit` of its mode? Used to
    /// skip storing runs that cannot reach the leaderboard.
    pub fn qualifies(&self, mode: &str, score: u32, limit: usize) -> rusqlite::Result<bool> {
        if score == 0 {
            return Ok(false);
        }
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM scores WHERE mode = ?1",
            (mode,),
            |r| r.get(0),
        )?;
        if (count as usize) < limit {
            return Ok(true);
        }
        let worst: i64 = self.conn.query_row(
            "SELECT score FROM scores WHERE mode = ?1 ORDER BY score DESC, id ASC LIMIT 1 OFFSET ?2",
            (mode, (limit - 1) as i64),
            |r| r.get(0),
        )?;
        Ok(i64::from(score) > worst)
    }

    /// Insert an entry, then prune the mode to its top `limit`.
    pub fn insert(
        &self,
        mode: &str,
        name: &str,
        score: u32,
        date: &str,
        limit: usize,
    ) -> rusqlite::Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.conn.execute(
            "INSERT INTO scores (mode, name, score, date, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (mode, name, i64::from(score), date, now),
        )?;
        self.conn.execute(
            "DELETE FROM scores WHERE mode = ?1 AND id NOT IN (
                 SELECT id FROM scores WHERE mode = ?1
                 ORDER BY score DESC, id ASC LIMIT ?2
             )",
            (mode, limit as i64),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_top_and_prune() {
        let db = Db::open(None).unwrap();
        for i in 0..15 {
            db.insert("m", &format!("p{i}"), i, "2026-06-13", 10)
                .unwrap();
        }
        let top = db.top("m", 10).unwrap();
        assert_eq!(top.len(), 10, "pruned to limit");
        assert_eq!(top[0].score, 14);
        assert_eq!(top[9].score, 5);
        assert!(top.windows(2).all(|w| w[0].score >= w[1].score));
    }

    #[test]
    fn qualification_rules() {
        let db = Db::open(None).unwrap();
        assert!(!db.qualifies("m", 0, 10).unwrap(), "zero never qualifies");
        assert!(db.qualifies("m", 1, 10).unwrap(), "empty table");
        for i in 10..20 {
            db.insert("m", "x", i, "d", 10).unwrap();
        }
        assert!(!db.qualifies("m", 9, 10).unwrap(), "below worst");
        assert!(!db.qualifies("m", 10, 10).unwrap(), "equal to worst");
        assert!(db.qualifies("m", 11, 10).unwrap());
    }

    #[test]
    fn modes_are_independent() {
        let db = Db::open(None).unwrap();
        db.insert("a", "x", 5, "d", 10).unwrap();
        db.insert("b", "y", 7, "d", 10).unwrap();
        assert_eq!(db.top("a", 10).unwrap().len(), 1);
        assert_eq!(db.top("b", 10).unwrap()[0].name, "y");
        assert!(db.top("c", 10).unwrap().is_empty());
    }
}
