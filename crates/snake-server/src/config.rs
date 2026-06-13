//! Server configuration, populated from environment variables.

use std::path::PathBuf;

/// All tunables of the server. Defaults are safe for a small public host;
/// every field has an environment override so the Docker image can be
/// configured without a rebuild.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// SQLite file. `None` opens an in-memory database (used by tests).
    pub db_path: Option<PathBuf>,
    /// Directory of static files (the built WASM frontend) served at `/`.
    /// `None` disables static serving (API-only).
    pub static_dir: Option<PathBuf>,
    /// Highscores kept per mode (also the leaderboard length).
    pub max_entries: usize,
    /// Hard cap on the number of recorded inputs in a submission.
    pub max_inputs: usize,
    /// Hard cap on ticks the re-simulation may run before giving up.
    pub max_ticks: u32,
    /// Maximum player-name length, in characters.
    pub max_name_len: usize,
    /// Request body size limit, in bytes.
    pub max_body_bytes: usize,
    /// Allowed POST submissions per client IP per minute.
    pub rate_limit_per_min: u32,
    /// Concurrent re-simulations allowed (CPU guard).
    pub verify_concurrency: usize,
    /// Trust the `X-Forwarded-For` header for the client IP (only enable
    /// behind a proxy/tunnel you control).
    pub trust_forwarded_for: bool,
    /// Secret mixed into the daily-challenge seed derivation.
    pub daily_secret: u64,
    /// Allowed CORS origins (for the GitHub-Pages + external-server setup).
    /// Empty disables CORS (same-origin Docker case needs none).
    pub cors_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            db_path: None,
            static_dir: None,
            max_entries: 10,
            max_inputs: 100_000,
            max_ticks: 2_000_000,
            max_name_len: 20,
            max_body_bytes: 512 * 1024,
            rate_limit_per_min: 20,
            verify_concurrency: 4,
            trust_forwarded_for: false,
            daily_secret: 0x5165_4361_6c63_5346, // "QeCalcSF"
            cors_origins: Vec::new(),
        }
    }
}

impl ServerConfig {
    /// Build from environment variables, falling back to [`Default`].
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(v) = std::env::var("DB_PATH") {
            cfg.db_path = Some(PathBuf::from(v));
        }
        if let Ok(v) = std::env::var("STATIC_DIR") {
            cfg.static_dir = Some(PathBuf::from(v));
        }
        if let Some(v) = env_parse("MAX_ENTRIES") {
            cfg.max_entries = v;
        }
        if let Some(v) = env_parse("MAX_INPUTS") {
            cfg.max_inputs = v;
        }
        if let Some(v) = env_parse("MAX_TICKS") {
            cfg.max_ticks = v;
        }
        if let Some(v) = env_parse("MAX_NAME_LEN") {
            cfg.max_name_len = v;
        }
        if let Some(v) = env_parse("MAX_BODY_BYTES") {
            cfg.max_body_bytes = v;
        }
        if let Some(v) = env_parse("RATE_LIMIT_PER_MIN") {
            cfg.rate_limit_per_min = v;
        }
        if let Some(v) = env_parse("VERIFY_CONCURRENCY") {
            cfg.verify_concurrency = v;
        }
        if let Ok(v) = std::env::var("TRUST_FORWARDED_FOR") {
            cfg.trust_forwarded_for = matches!(v.as_str(), "1" | "true" | "yes");
        }
        if let Some(v) = env_parse("DAILY_SECRET") {
            cfg.daily_secret = v;
        }
        if let Ok(v) = std::env::var("CORS_ALLOW_ORIGIN") {
            cfg.cors_origins = v
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
        }
        cfg
    }
}

fn env_parse<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|v| v.parse().ok())
}
