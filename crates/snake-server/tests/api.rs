//! End-to-end tests of the HTTP API via `tower::ServiceExt::oneshot`.
//!
//! These cover the happy path (a verified run lands on the leaderboard) and
//! the hardening requirements: overlong names, overlong input lists and
//! inconsistent runs are all rejected with 4xx.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use snake_core::api::ScoreSubmission;
use snake_core::{BoundaryMode, Config, Direction, GameState, Recorder, Status};
use snake_server::config::ServerConfig;
use snake_server::{router, AppState};
use tower::ServiceExt;

const MODE: &str = "walls-16x12-normal";

fn test_state() -> AppState {
    let cfg = ServerConfig {
        // Disable rate limiting in tests so repeated POSTs are allowed.
        rate_limit_per_min: 0,
        ..Default::default()
    };
    AppState::new(cfg).unwrap()
}

/// Play a greedy run on the given board and capture it as a submission.
fn record_run(
    width: i32,
    height: i32,
    boundary: BoundaryMode,
    seed: u64,
) -> (ScoreSubmission, u32) {
    let config = Config {
        width,
        height,
        boundary,
        seed,
    };
    let mut state = GameState::new(config);
    let mut rec = Recorder::new();
    let mut ticks = 0;
    while state.status() == Status::Running && ticks < 50_000 {
        let food = state.food();
        let board = *state.board();
        let head = state.head();
        let input = Direction::ALL
            .into_iter()
            .filter(|d| *d != state.direction().opposite())
            .filter_map(|d| board.neighbor(head, d).map(|n| (d, n)))
            .filter(|(_, n)| !state.occupies(*n) || *n == state.tail())
            .min_by_key(|(_, n)| board.distance(*n, food))
            .map(|(d, _)| d);
        rec.record(input);
        state.tick(input);
        ticks += 1;
    }
    let score = state.score();
    let replay = rec.into_replay(config);
    (
        ScoreSubmission {
            name: "Tester".to_owned(),
            claimed_score: score,
            replay,
            signature: None,
        },
        score,
    )
}

async fn post(state: &AppState, path: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = router(state.clone()).oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

async fn get(state: &AppState, path: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .unwrap();
    let resp = router(state.clone()).oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

#[tokio::test]
async fn valid_run_lands_on_leaderboard() {
    let state = test_state();
    let (sub, score) = record_run(16, 12, BoundaryMode::Walls, 7);
    assert!(score > 0, "greedy run should score");

    let (status, body) = post(&state, &format!("/highscores/{MODE}"), json!(sub)).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["entries"][0]["score"], score);
    assert_eq!(body["entries"][0]["name"], "Tester");

    // It also shows up via GET.
    let (status, body) = get(&state, &format!("/highscores/{MODE}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["entries"][0]["score"], score);
}

#[tokio::test]
async fn server_derives_score_ignoring_inflated_claim() {
    let state = test_state();
    let (mut sub, score) = record_run(16, 12, BoundaryMode::Walls, 7);
    sub.claimed_score = 9999; // lie
    let (status, body) = post(&state, &format!("/highscores/{MODE}"), json!(sub)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["entries"][0]["score"], score,
        "server trusts re-simulation, not the claim"
    );
}

#[tokio::test]
async fn rejects_unknown_mode() {
    let state = test_state();
    let (sub, _) = record_run(16, 12, BoundaryMode::Walls, 7);
    let (status, _) = post(&state, "/highscores/walls-20x15-normal", json!(sub)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn rejects_overlong_name() {
    let state = test_state();
    let (mut sub, _) = record_run(16, 12, BoundaryMode::Walls, 7);
    sub.name = "x".repeat(100);
    let (status, body) = post(&state, &format!("/highscores/{MODE}"), json!(sub)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("name"));
}

#[tokio::test]
async fn rejects_overlong_input_list() {
    let cfg = ServerConfig {
        rate_limit_per_min: 0,
        max_inputs: 5,
        ..Default::default()
    };
    let state = AppState::new(cfg).unwrap();
    let (sub, _) = record_run(16, 12, BoundaryMode::Walls, 7);
    assert!(sub.replay.inputs.len() > 5, "test needs a longer run");
    let (status, body) = post(&state, &format!("/highscores/{MODE}"), json!(sub)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("inputs"));
}

#[tokio::test]
async fn rejects_inconsistent_run() {
    let state = test_state();
    let (mut sub, _) = record_run(16, 12, BoundaryMode::Walls, 7);
    // Inject an input far past the end of the run: re-simulation never
    // consumes it, so verification fails.
    sub.replay.inputs.push((9_000_000, Direction::NorthEast));
    let (status, _) = post(&state, &format!("/highscores/{MODE}"), json!(sub)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn rejects_board_mismatch() {
    let state = test_state();
    // Replay on a different preset than the mode path claims.
    let (sub, _) = record_run(24, 18, BoundaryMode::Walls, 7);
    let (status, _) = post(&state, &format!("/highscores/{MODE}"), json!(sub)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn rate_limit_blocks_after_threshold() {
    let cfg = ServerConfig {
        rate_limit_per_min: 2,
        ..Default::default()
    };
    let state = AppState::new(cfg).unwrap();
    let (sub, _) = record_run(16, 12, BoundaryMode::Walls, 7);
    let path = format!("/highscores/{MODE}");
    let (s1, _) = post(&state, &path, json!(sub)).await;
    let (s2, _) = post(&state, &path, json!(sub)).await;
    let (s3, _) = post(&state, &path, json!(sub)).await;
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(s3, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn daily_challenge_roundtrip() {
    let state = test_state();
    let (status, challenge) = get(&state, "/challenge").await;
    assert_eq!(status, StatusCode::OK);
    let seed = challenge["seed"].as_u64().unwrap();
    let width = challenge["width"].as_i64().unwrap() as i32;
    let height = challenge["height"].as_i64().unwrap() as i32;

    let (sub, score) = record_run(width, height, BoundaryMode::Walls, seed);
    let (status, body) = post(&state, "/challenge/highscores", json!(sub)).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    if score > 0 {
        assert_eq!(body["entries"][0]["score"], score);
    }

    let (status, body) = get(&state, "/challenge/highscores").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["mode"].as_str().unwrap().starts_with("daily-"));
}

#[tokio::test]
async fn daily_rejects_wrong_seed() {
    let state = test_state();
    // A run with an arbitrary seed that is (almost certainly) not today's.
    let (sub, _) = record_run(24, 18, BoundaryMode::Walls, 123_456);
    let (status, _) = post(&state, "/challenge/highscores", json!(sub)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
