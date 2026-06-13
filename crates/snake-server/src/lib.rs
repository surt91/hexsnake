//! Optional global-highscore server for HexSnake.
//!
//! axum + SQLite. Submissions carry a [`Replay`](snake_core::Replay) (seed +
//! input list); the server re-simulates it with `snake-core` and trusts only
//! its own derived score. The public endpoints are hardened: body-size limit,
//! input validation, per-IP rate limiting and a concurrency cap on the
//! (CPU-bound) re-simulation.

pub mod config;
pub mod daily;
pub mod db;
pub mod mode;
pub mod ratelimit;

use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};

use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, FromRequestParts, Path, State},
    http::{request::Parts, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use snake_core::api::{DailyChallenge, Leaderboard, ScoreSubmission};
use snake_core::{Replay, Status, VerifiedRun};
use tokio::sync::Semaphore;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use config::ServerConfig;
use db::Db;
use mode::ModeBoard;
use ratelimit::RateLimiter;

/// Shared server state.
#[derive(Clone)]
pub struct AppState {
    cfg: Arc<ServerConfig>,
    db: Arc<Mutex<Db>>,
    verify: Arc<Semaphore>,
    rate: Arc<Mutex<RateLimiter>>,
}

impl AppState {
    pub fn new(cfg: ServerConfig) -> rusqlite::Result<Self> {
        let db = Db::open(cfg.db_path.as_deref())?;
        let rate = RateLimiter::per_minute(cfg.rate_limit_per_min);
        let verify = Semaphore::new(cfg.verify_concurrency.max(1));
        Ok(Self {
            verify: Arc::new(verify),
            rate: Arc::new(Mutex::new(rate)),
            db: Arc::new(Mutex::new(db)),
            cfg: Arc::new(cfg),
        })
    }
}

/// Build the application router for the given state.
pub fn router(state: AppState) -> Router {
    let cfg = state.cfg.clone();
    let mut app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route(
            "/highscores/{mode}",
            get(get_highscores).post(post_highscores),
        )
        .route("/challenge", get(get_challenge))
        .route(
            "/challenge/highscores",
            get(get_daily_highscores).post(post_daily_highscores),
        );

    if let Some(dir) = cfg.static_dir.clone() {
        // The axum server can also serve the built WASM frontend, so the
        // whole game ships as one container.
        app = app.fallback_service(ServeDir::new(dir));
    }

    let app = app
        .layer(DefaultBodyLimit::max(cfg.max_body_bytes))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    if cfg.cors_origins.is_empty() {
        app
    } else {
        let origins: Vec<_> = cfg
            .cors_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        let cors = CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods([Method::GET, Method::POST])
            .allow_headers([axum::http::header::CONTENT_TYPE]);
        app.layer(cors)
    }
}

// --- client IP extractor ----------------------------------------------------

/// The resolved client IP for rate limiting. Trusts `X-Forwarded-For` only
/// when configured (i.e. behind a proxy/tunnel we control), otherwise uses
/// the peer socket address. Falls back to `0.0.0.0` when neither is present
/// (e.g. in `oneshot` tests).
pub struct ClientIp(pub IpAddr);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if state.cfg.trust_forwarded_for {
            if let Some(ip) = parts
                .headers
                .get("x-forwarded-for")
                .and_then(|h| h.to_str().ok())
                .and_then(|v| v.split(',').next())
                .and_then(|s| s.trim().parse().ok())
            {
                return Ok(ClientIp(ip));
            }
        }
        let ip = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip())
            .unwrap_or(IpAddr::from([0, 0, 0, 0]));
        Ok(ClientIp(ip))
    }
}

// --- error helper -----------------------------------------------------------

/// JSON error response with an explicit status code.
struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

fn bad(msg: &str) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.to_owned())
}

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("internal: {e}"))
}

// --- handlers ---------------------------------------------------------------

async fn get_highscores(
    State(state): State<AppState>,
    Path(mode): Path<String>,
) -> Result<Json<Leaderboard>, ApiError> {
    mode::parse(&mode).ok_or_else(|| bad("unknown mode"))?;
    let entries = state
        .db
        .lock()
        .unwrap()
        .top(&mode, state.cfg.max_entries)
        .map_err(internal)?;
    Ok(Json(Leaderboard { mode, entries }))
}

async fn post_highscores(
    State(state): State<AppState>,
    Path(mode): Path<String>,
    ClientIp(ip): ClientIp,
    Json(sub): Json<ScoreSubmission>,
) -> Result<Json<Leaderboard>, ApiError> {
    let board = mode::parse(&mode).ok_or_else(|| bad("unknown mode"))?;
    let leaderboard = store_run(&state, mode, board, None, ip, sub).await?;
    Ok(Json(leaderboard))
}

async fn get_challenge(State(state): State<AppState>) -> Json<DailyChallenge> {
    let days = daily::unix_days();
    Json(DailyChallenge {
        date: daily::iso_from_days(days),
        seed: daily::seed_for_day(days, state.cfg.daily_secret),
        width: daily::DAILY_WIDTH,
        height: daily::DAILY_HEIGHT,
        boundary: daily::DAILY_BOUNDARY,
    })
}

async fn get_daily_highscores(
    State(state): State<AppState>,
) -> Result<Json<Leaderboard>, ApiError> {
    let mode = daily_mode(&daily::today_iso());
    let entries = state
        .db
        .lock()
        .unwrap()
        .top(&mode, state.cfg.max_entries)
        .map_err(internal)?;
    Ok(Json(Leaderboard { mode, entries }))
}

async fn post_daily_highscores(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Json(sub): Json<ScoreSubmission>,
) -> Result<Json<Leaderboard>, ApiError> {
    let days = daily::unix_days();
    let seed = daily::seed_for_day(days, state.cfg.daily_secret);
    let board = ModeBoard {
        boundary: daily::DAILY_BOUNDARY,
        width: daily::DAILY_WIDTH,
        height: daily::DAILY_HEIGHT,
    };
    let mode = daily_mode(&daily::iso_from_days(days));
    let leaderboard = store_run(&state, mode, board, Some(seed), ip, sub).await?;
    Ok(Json(leaderboard))
}

// --- shared submission logic ------------------------------------------------

fn daily_mode(date: &str) -> String {
    format!("daily-{date}")
}

/// Rate-limit, validate, verify and store a submission, returning the updated
/// leaderboard. `expected` is the board the mode implies; `expected_seed`, if
/// set, pins the seed (daily challenge).
async fn store_run(
    state: &AppState,
    mode: String,
    expected: ModeBoard,
    expected_seed: Option<u64>,
    ip: IpAddr,
    sub: ScoreSubmission,
) -> Result<Leaderboard, ApiError> {
    // Cheap rejections first, before any CPU-bound work.
    if !state.rate.lock().unwrap().check(ip) {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "rate limit exceeded".to_owned(),
        ));
    }
    let name = validate_name(&state.cfg, &sub.name)?;
    if sub.replay.inputs.len() > state.cfg.max_inputs {
        return Err(bad("too many inputs"));
    }
    let r = &sub.replay;
    if r.width != expected.width || r.height != expected.height || r.boundary != expected.boundary {
        return Err(bad("replay board does not match mode"));
    }
    if let Some(seed) = expected_seed {
        if r.seed != seed {
            return Err(bad("replay seed does not match challenge"));
        }
    }

    let verified = verify_replay(state, sub.replay).await?;
    debug_assert_ne!(verified.status, Status::Running);
    let score = verified.score;
    let date = daily::today_iso();

    let db = state.db.lock().unwrap();
    if db
        .qualifies(&mode, score, state.cfg.max_entries)
        .map_err(internal)?
    {
        db.insert(&mode, &name, score, &date, state.cfg.max_entries)
            .map_err(internal)?;
    }
    let entries = db.top(&mode, state.cfg.max_entries).map_err(internal)?;
    Ok(Leaderboard { mode, entries })
}

/// Re-simulate a replay under the concurrency cap, off the async runtime.
async fn verify_replay(state: &AppState, replay: Replay) -> Result<VerifiedRun, ApiError> {
    let _permit = state
        .verify
        .clone()
        .acquire_owned()
        .await
        .map_err(internal)?;
    let max_ticks = state.cfg.max_ticks;
    tokio::task::spawn_blocking(move || replay.verify(max_ticks))
        .await
        .map_err(internal)?
        .ok_or_else(|| bad("inconsistent or non-terminating run"))
}

fn validate_name(cfg: &ServerConfig, name: &str) -> Result<String, ApiError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(bad("name must not be empty"));
    }
    if trimmed.chars().count() > cfg.max_name_len {
        return Err(bad("name too long"));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(bad("name contains control characters"));
    }
    Ok(trimmed.to_owned())
}
