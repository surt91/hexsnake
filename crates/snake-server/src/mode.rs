//! Validation of mode keys for the global leaderboards.
//!
//! Global leaderboards exist only for the three size presets crossed with
//! both boundaries and the three speeds: `"{boundary}-{w}x{h}-{speed}"`,
//! e.g. `"walls-16x12-normal"` or `"torus-32x24-fast"`. The speed only
//! selects the table; it does not affect the deterministic simulation, so it
//! is not part of the replay and is taken on trust.

use snake_core::BoundaryMode;

/// The board a mode key implies, used to cross-check a submitted replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeBoard {
    pub boundary: BoundaryMode,
    pub width: i32,
    pub height: i32,
}

/// The three allowed preset dimensions (Hamilton-compatible).
const PRESETS: [(i32, i32); 3] = [(16, 12), (24, 18), (32, 24)];
const SPEEDS: [&str; 3] = ["slow", "normal", "fast"];

/// Parse and validate a global-leaderboard mode key. Returns the implied
/// board, or `None` if the key is not one of the allowed preset modes.
pub fn parse(mode: &str) -> Option<ModeBoard> {
    let mut parts = mode.split('-');
    let boundary = match parts.next()? {
        "walls" => BoundaryMode::Walls,
        "torus" => BoundaryMode::Periodic,
        _ => return None,
    };
    let size = parts.next()?;
    let speed = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if !SPEEDS.contains(&speed) {
        return None;
    }
    let (w, h) = size.split_once('x')?;
    let (w, h): (i32, i32) = (w.parse().ok()?, h.parse().ok()?);
    if !PRESETS.contains(&(w, h)) {
        return None;
    }
    Some(ModeBoard {
        boundary,
        width: w,
        height: h,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_all_valid_modes() {
        for b in ["walls", "torus"] {
            for (w, h) in PRESETS {
                for s in SPEEDS {
                    let key = format!("{b}-{w}x{h}-{s}");
                    assert!(parse(&key).is_some(), "should accept {key}");
                }
            }
        }
    }

    #[test]
    fn rejects_non_presets_and_garbage() {
        assert!(parse("walls-20x15-normal").is_none(), "custom size");
        assert!(parse("walls-16x12-turbo").is_none(), "bad speed");
        assert!(parse("diag-16x12-normal").is_none(), "bad boundary");
        assert!(parse("walls-16x12").is_none(), "missing speed");
        assert!(parse("walls-16x12-normal-extra").is_none(), "trailing");
        assert!(parse("").is_none());
    }

    #[test]
    fn implied_board_matches_key() {
        let board = parse("torus-24x18-fast").unwrap();
        assert_eq!(board.boundary, BoundaryMode::Periodic);
        assert_eq!((board.width, board.height), (24, 18));
    }
}
