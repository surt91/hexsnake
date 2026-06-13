//! Recording and verification of a complete game run.
//!
//! A run is fully determined by its [`Config`] (board + seed) and the
//! sequence of inputs fed to [`GameState::tick`]. Recording those inputs
//! sparsely — only the ticks that carried a non-`None` input — lets a
//! server re-simulate the run with this very crate and verify a
//! client-claimed score without trusting the client (same crate ⇒ same
//! behavior).
//!
//! The format is deliberately plain (board dimensions, boundary, seed and a
//! `(tick, direction)` list) so it serializes to compact JSON and is easy to
//! reason about. It is also the foundation for replays/ghosts later on.

use crate::board::BoundaryMode;
use crate::coords::Direction;
use crate::game::{Config, GameState, Status};

/// Records the inputs of a running game tick by tick, so the run can be
/// turned into a [`Replay`] for submission.
#[derive(Debug, Clone, Default)]
pub struct Recorder {
    tick: u32,
    inputs: Vec<(u32, Direction)>,
}

impl Recorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Note the input that is about to be fed to [`GameState::tick`]. Call
    /// exactly once per `tick`, in order, with the same value passed to the
    /// game.
    pub fn record(&mut self, input: Option<Direction>) {
        if let Some(dir) = input {
            self.inputs.push((self.tick, dir));
        }
        self.tick = self.tick.saturating_add(1);
    }

    /// Number of ticks recorded so far.
    pub fn ticks(&self) -> u32 {
        self.tick
    }

    /// Build a submittable replay from the recorded inputs and the config the
    /// game was started with.
    pub fn into_replay(self, config: Config) -> Replay {
        Replay {
            width: config.width,
            height: config.height,
            boundary: config.boundary,
            seed: config.seed,
            inputs: self.inputs,
        }
    }
}

/// A complete, re-simulatable run: board parameters, seed and the sparse
/// list of inputs. Verifying it replays the run deterministically.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Replay {
    pub width: i32,
    pub height: i32,
    pub boundary: BoundaryMode,
    pub seed: u64,
    /// Sparse, strictly increasing by tick: `(tick_index, input)` for the
    /// ticks where the player or strategy supplied an input.
    pub inputs: Vec<(u32, Direction)>,
}

/// Outcome of a verified run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedRun {
    pub score: u32,
    pub ticks: u32,
    pub status: Status,
}

impl Replay {
    pub fn config(&self) -> Config {
        Config {
            width: self.width,
            height: self.height,
            boundary: self.boundary,
            seed: self.seed,
        }
    }

    /// Re-simulate the run and return its outcome, or `None` if the replay is
    /// malformed or could not be played out within `max_ticks`.
    ///
    /// The score in the result is authoritative — the server trusts this over
    /// any value the client claimed. Rejected as `None`:
    /// - degenerate board dimensions (too small to host the snake),
    /// - inputs not strictly increasing in tick index,
    /// - an input scheduled for a tick the game never reaches (e.g. after it
    ///   already ended, or out of order),
    /// - a run that does not terminate within `max_ticks`.
    pub fn verify(&self, max_ticks: u32) -> Option<VerifiedRun> {
        // Guard against degenerate boards before `GameState::new` (which
        // asserts on boards too small for the starting snake).
        if self.width < 2 || self.height < 2 {
            return None;
        }
        let cells = (self.width as i64) * (self.height as i64);
        if cells <= 4 {
            return None;
        }

        // Inputs must be strictly increasing in tick index, so each maps to a
        // distinct tick and the consume-on-match logic below is unambiguous.
        if self.inputs.windows(2).any(|w| w[0].0 >= w[1].0) {
            return None;
        }

        let mut state = GameState::new(self.config());
        let mut pending = self.inputs.iter().copied().peekable();
        let mut tick = 0u32;
        while state.status() == Status::Running {
            if tick >= max_ticks {
                return None;
            }
            let input = match pending.peek() {
                Some(&(t, dir)) if t == tick => {
                    pending.next();
                    Some(dir)
                }
                _ => None,
            };
            state.tick(input);
            tick += 1;
        }

        // Any leftover input was scheduled past the end of the run: reject,
        // because a faithful client never records inputs the game ignores.
        if pending.next().is_some() {
            return None;
        }

        Some(VerifiedRun {
            score: state.score(),
            ticks: tick,
            status: state.status(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Config {
        Config {
            width: 16,
            height: 12,
            boundary: BoundaryMode::Walls,
            seed: 7,
        }
    }

    /// Greedy walker that records its inputs, used to produce a non-trivial
    /// run with a positive score.
    fn play_and_record(config: Config) -> (Replay, VerifiedRun) {
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
        let live = VerifiedRun {
            score: state.score(),
            ticks: rec.ticks(),
            status: state.status(),
        };
        (rec.into_replay(config), live)
    }

    #[test]
    fn verified_score_matches_live_run() {
        let (replay, live) = play_and_record(cfg());
        assert!(live.score > 0, "greedy walker should have eaten");
        let verified = replay.verify(100_000).expect("valid run verifies");
        assert_eq!(verified.score, live.score);
        assert_eq!(verified.ticks, live.ticks);
        assert_eq!(verified.status, live.status);
    }

    #[test]
    fn verification_is_deterministic_across_boundaries() {
        for boundary in [BoundaryMode::Walls, BoundaryMode::Periodic] {
            let config = Config { boundary, ..cfg() };
            let (replay, live) = play_and_record(config);
            let again = replay.verify(100_000).unwrap();
            assert_eq!(again.score, live.score, "boundary {boundary:?}");
        }
    }

    #[test]
    fn tampered_score_cannot_be_forged() {
        // The replay carries no score field — the server derives it. A client
        // claiming a higher score simply submits the same inputs, and
        // re-simulation yields the true (lower) score. Here we assert the
        // structural property: identical inputs always verify to one score.
        let (replay, _) = play_and_record(cfg());
        let a = replay.verify(100_000).unwrap();
        let b = replay.clone().verify(100_000).unwrap();
        assert_eq!(a.score, b.score);
    }

    #[test]
    fn rejects_non_increasing_inputs() {
        let replay = Replay {
            width: 16,
            height: 12,
            boundary: BoundaryMode::Walls,
            seed: 0,
            inputs: vec![(5, Direction::NorthEast), (5, Direction::SouthEast)],
        };
        assert!(replay.verify(10_000).is_none());
    }

    #[test]
    fn rejects_input_after_game_end() {
        // A wall run dies after `height` straight ticks; an input scheduled
        // far beyond that is never consumed and must be rejected.
        let replay = Replay {
            width: 16,
            height: 12,
            boundary: BoundaryMode::Walls,
            seed: 0,
            inputs: vec![(10_000, Direction::NorthEast)],
        };
        assert!(replay.verify(100_000).is_none());
    }

    #[test]
    fn rejects_degenerate_board() {
        let replay = Replay {
            width: 1,
            height: 1,
            boundary: BoundaryMode::Walls,
            seed: 0,
            inputs: vec![],
        };
        assert!(replay.verify(10_000).is_none());
    }

    #[test]
    fn rejects_runs_exceeding_tick_budget() {
        // A torus run going straight never dies; with a tiny budget it must
        // be rejected rather than looping forever.
        let replay = Replay {
            width: 16,
            height: 12,
            boundary: BoundaryMode::Periodic,
            seed: 0,
            inputs: vec![],
        };
        assert!(replay.verify(50).is_none());
    }
}
