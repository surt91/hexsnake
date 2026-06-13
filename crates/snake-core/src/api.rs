//! Wire types shared by the client and the optional highscore server.
//!
//! Kept in `snake-core` (behind the `serde` feature) so both sides use one
//! definition and the JSON schema cannot drift. These are plain data — the
//! game logic does not depend on them.

use crate::board::BoundaryMode;
use crate::replay::Replay;

/// Client → server: a finished run submitted to a global leaderboard.
///
/// `claimed_score` is informational only; the server re-simulates the
/// [`Replay`] and trusts its own derived score. `signature` is an optional
/// field reserved for a later keypair scheme — present from the start so it
/// can be filled in without a schema migration.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScoreSubmission {
    pub name: String,
    pub claimed_score: u32,
    pub replay: Replay,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// One global leaderboard entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScoreEntry {
    pub name: String,
    pub score: u32,
    /// ISO date (UTC), e.g. "2026-06-13".
    pub date: String,
}

/// Server → client: a leaderboard for one mode (or daily date).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Leaderboard {
    pub mode: String,
    pub entries: Vec<ScoreEntry>,
}

/// Server → client: today's daily challenge.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DailyChallenge {
    /// ISO date (UTC) the seed is valid for, "YYYY-MM-DD".
    pub date: String,
    pub seed: u64,
    /// Board the daily challenge is played on.
    pub width: i32,
    pub height: i32,
    pub boundary: BoundaryMode,
}
