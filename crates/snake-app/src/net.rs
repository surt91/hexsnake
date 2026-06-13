//! Optional networking for the global highscore server.
//!
//! Everything here is best-effort and non-blocking: requests run via `ehttp`
//! (native + WASM) and their results arrive on a channel that the UI drains
//! each frame. A missing or unreachable server never blocks or breaks the
//! game — global tables simply stay empty, and a run made while offline is
//! kept in a persisted pending queue and retried later. With the network
//! unplugged the app behaves exactly as the offline-only build did.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use snake_core::api::{DailyChallenge, Leaderboard, ScoreSubmission};

/// A run awaiting (re)submission. Persisted so an offline run is not lost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSubmission {
    pub id: u64,
    /// Leaderboard key the run belongs to (also the cache key).
    pub mode: String,
    /// `true` for the daily challenge endpoint, `false` for a preset mode.
    pub daily: bool,
    pub submission: ScoreSubmission,
}

/// Messages handed back from background requests to the UI thread.
enum Inbox {
    Fetched {
        mode: String,
        board: Leaderboard,
    },
    Submitted {
        id: u64,
        mode: String,
        board: Leaderboard,
    },
    SubmitFailed {
        id: u64,
    },
    Challenge(DailyChallenge),
}

/// Connection hint shown in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reachability {
    Unknown,
    Online,
    Offline,
}

pub struct Online {
    /// Server base URL, or `None` when networking is disabled (no server
    /// configured on a native build).
    base: Option<String>,
    tx: Sender<Inbox>,
    rx: Receiver<Inbox>,
    boards: HashMap<String, Leaderboard>,
    fetching: HashSet<String>,
    in_flight: HashSet<u64>,
    pending: Vec<PendingSubmission>,
    next_id: u64,
    challenge: Option<DailyChallenge>,
    reachability: Reachability,
    /// egui timestamp of the last queue retry, to throttle re-sends.
    last_retry: f64,
}

const TIMEOUT: Duration = Duration::from_secs(8);
/// Minimum seconds between retries of the pending queue.
const RETRY_INTERVAL: f64 = 15.0;

impl Online {
    /// Build from the resolved server base and any persisted pending runs.
    pub fn new(pending: Vec<PendingSubmission>) -> Self {
        let (tx, rx) = channel();
        let next_id = pending.iter().map(|p| p.id + 1).max().unwrap_or(0);
        Self {
            base: server_base(),
            tx,
            rx,
            boards: HashMap::new(),
            fetching: HashSet::new(),
            in_flight: HashSet::new(),
            pending,
            next_id,
            challenge: None,
            reachability: Reachability::Unknown,
            last_retry: f64::NEG_INFINITY,
        }
    }

    /// Whether a server is configured at all (controls whether to show any
    /// global UI).
    pub fn enabled(&self) -> bool {
        self.base.is_some()
    }

    pub fn reachability(&self) -> Reachability {
        self.reachability
    }

    pub fn pending(&self) -> &[PendingSubmission] {
        &self.pending
    }

    pub fn board(&self, mode: &str) -> Option<&Leaderboard> {
        self.boards.get(mode)
    }

    pub fn challenge(&self) -> Option<&DailyChallenge> {
        self.challenge.as_ref()
    }

    fn url(&self, path: &str) -> Option<String> {
        let base = self.base.as_ref()?;
        Some(format!("{}/{}", base.trim_end_matches('/'), path))
    }

    /// Drain background results into local state. Call once per frame.
    pub fn pump(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Inbox::Fetched { mode, board } => {
                    self.reachability = Reachability::Online;
                    self.boards.insert(mode.clone(), board);
                    self.fetching.remove(&mode);
                }
                Inbox::Submitted { id, mode, board } => {
                    self.reachability = Reachability::Online;
                    self.boards.insert(mode, board);
                    self.in_flight.remove(&id);
                    self.pending.retain(|p| p.id != id);
                }
                Inbox::SubmitFailed { id } => {
                    self.reachability = Reachability::Offline;
                    self.in_flight.remove(&id);
                    // Stays in `pending` for a later retry.
                }
                Inbox::Challenge(c) => {
                    self.reachability = Reachability::Online;
                    self.challenge = Some(c);
                }
            }
        }
    }

    /// Request the leaderboard for `mode` unless it is already cached or a
    /// fetch is already running.
    pub fn fetch_board(&mut self, ctx: &eframe::egui::Context, mode: &str) {
        if self.base.is_none() || self.fetching.contains(mode) || self.boards.contains_key(mode) {
            return;
        }
        let Some(url) = self.url(&format!("highscores/{mode}")) else {
            return;
        };
        self.fetching.insert(mode.to_owned());
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let mode = mode.to_owned();
        let mut request = ehttp::Request::get(url);
        request.timeout = Some(TIMEOUT);
        ehttp::fetch(request, move |result| {
            if let Some(board) = parse_board(result) {
                let _ = tx.send(Inbox::Fetched { mode, board });
            }
            ctx.request_repaint();
        });
    }

    /// Fetch today's daily challenge (seed + board) and its leaderboard.
    pub fn fetch_challenge(&mut self, ctx: &eframe::egui::Context) {
        let Some(url) = self.url("challenge") else {
            return;
        };
        let tx = self.tx.clone();
        let ctx2 = ctx.clone();
        let mut request = ehttp::Request::get(url);
        request.timeout = Some(TIMEOUT);
        ehttp::fetch(request, move |result| {
            if let Ok(resp) = result {
                if let Ok(c) = serde_json::from_slice::<DailyChallenge>(&resp.bytes) {
                    let _ = tx.send(Inbox::Challenge(c));
                }
            }
            ctx2.request_repaint();
        });
        self.fetch_board_path(ctx, "daily", "challenge/highscores");
    }

    fn fetch_board_path(&mut self, ctx: &eframe::egui::Context, cache_key: &str, path: &str) {
        if self.base.is_none()
            || self.fetching.contains(cache_key)
            || self.boards.contains_key(cache_key)
        {
            return;
        }
        let Some(url) = self.url(path) else { return };
        self.fetching.insert(cache_key.to_owned());
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let cache_key = cache_key.to_owned();
        let mut request = ehttp::Request::get(url);
        request.timeout = Some(TIMEOUT);
        ehttp::fetch(request, move |result| {
            if let Some(board) = parse_board(result) {
                let _ = tx.send(Inbox::Fetched {
                    mode: cache_key,
                    board,
                });
            }
            ctx.request_repaint();
        });
    }

    /// Refresh the daily-challenge leaderboard (key `"daily"`).
    pub fn fetch_daily_board(&mut self, ctx: &eframe::egui::Context) {
        self.fetch_board_path(ctx, "daily", "challenge/highscores");
    }

    /// Queue a run for submission and send it immediately (if online).
    pub fn submit(
        &mut self,
        ctx: &eframe::egui::Context,
        mode: String,
        daily: bool,
        submission: ScoreSubmission,
    ) {
        let id = self.next_id;
        self.next_id += 1;
        let job = PendingSubmission {
            id,
            mode,
            daily,
            submission,
        };
        self.pending.push(job.clone());
        if self.base.is_some() {
            self.send(ctx, job);
        }
    }

    /// Retry the pending queue, throttled to at most once per
    /// [`RETRY_INTERVAL`]. `now` is the egui timestamp.
    pub fn retry_pending(&mut self, ctx: &eframe::egui::Context, now: f64) {
        if self.base.is_none() || self.pending.is_empty() {
            return;
        }
        if now - self.last_retry < RETRY_INTERVAL {
            return;
        }
        self.last_retry = now;
        let jobs: Vec<PendingSubmission> = self
            .pending
            .iter()
            .filter(|p| !self.in_flight.contains(&p.id))
            .cloned()
            .collect();
        for job in jobs {
            self.send(ctx, job);
        }
    }

    fn send(&mut self, ctx: &eframe::egui::Context, job: PendingSubmission) {
        let path = if job.daily {
            "challenge/highscores".to_owned()
        } else {
            format!("highscores/{}", job.mode)
        };
        let Some(url) = self.url(&path) else { return };
        let Ok(body) = serde_json::to_vec(&job.submission) else {
            return;
        };
        self.in_flight.insert(job.id);
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let mut request = ehttp::Request::post(url, body);
        request.headers.insert("Content-Type", "application/json");
        request.timeout = Some(TIMEOUT);
        ehttp::fetch(request, move |result| {
            match parse_board(result) {
                Some(board) => {
                    let _ = tx.send(Inbox::Submitted {
                        id: job.id,
                        mode: job.mode,
                        board,
                    });
                }
                None => {
                    let _ = tx.send(Inbox::SubmitFailed { id: job.id });
                }
            }
            ctx.request_repaint();
        });
    }
}

/// Parse a successful leaderboard response, or `None` on any error/4xx.
/// A 4xx (e.g. the server rejected an inconsistent run) is treated as a
/// permanent failure for fetches; for submissions it still drops the run
/// from the queue via the `Submitted`/`SubmitFailed` split in the caller —
/// here we only decode 2xx bodies.
fn parse_board(result: ehttp::Result<ehttp::Response>) -> Option<Leaderboard> {
    let resp = result.ok()?;
    if !resp.ok {
        return None;
    }
    serde_json::from_slice::<Leaderboard>(&resp.bytes).ok()
}

/// Resolve the server base URL.
///
/// 1. A compile-time `SNAKE_SERVER_URL` wins (set for the GitHub-Pages build
///    to point at an external API server).
/// 2. Otherwise, on the web, default to same-origin relative URLs — this is
///    the all-in-one Docker case where the server also serves the frontend.
/// 3. On native, only a runtime `SNAKE_SERVER_URL` enables networking.
fn server_base() -> Option<String> {
    if let Some(url) = option_env!("SNAKE_SERVER_URL") {
        let url = url.trim();
        if !url.is_empty() {
            return Some(url.to_owned());
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        Some(String::new())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var("SNAKE_SERVER_URL")
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    }
}
