//! Headless benchmark harness: play N games per strategy and report
//! average score and survival time.

use crate::game::{Config, GameState, Status};
use crate::strategy::Strategy;

#[derive(Debug, Clone, Copy)]
pub struct GameResult {
    pub score: u32,
    pub ticks: u64,
    pub status: Status,
}

/// Play one full game headless. `max_ticks` caps strategies that can stall
/// forever (e.g. tail chasing).
pub fn play_game(strategy: &mut dyn Strategy, config: Config, max_ticks: u64) -> GameResult {
    let mut state = GameState::new(config);
    while state.status() == Status::Running && state.ticks() < max_ticks {
        let dir = strategy.next_move(&state);
        state.tick(Some(dir));
    }
    GameResult {
        score: state.score(),
        ticks: state.ticks(),
        status: state.status(),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Summary {
    pub games: u64,
    pub avg_score: f64,
    pub avg_ticks: f64,
    pub max_score: u32,
    /// Fraction of games that ended with [`Status::Won`] (board full).
    pub won_frac: f64,
    /// Mean ticks over the won games only; `None` if none were won.
    pub avg_ticks_won: Option<f64>,
}

/// Run `games` games with seeds `0..games`; the strategy is rebuilt per
/// game via `make_strategy(seed)` so seeded strategies stay deterministic.
pub fn run_series(
    make_strategy: &mut dyn FnMut(u64) -> Box<dyn Strategy>,
    base: Config,
    games: u64,
    max_ticks: u64,
) -> Summary {
    let mut total_score = 0u64;
    let mut total_ticks = 0u64;
    let mut max_score = 0u32;
    let mut won_games = 0u64;
    let mut won_ticks = 0u64;
    for seed in 0..games {
        let config = Config { seed, ..base };
        let mut strategy = make_strategy(seed);
        let result = play_game(strategy.as_mut(), config, max_ticks);
        total_score += u64::from(result.score);
        total_ticks += result.ticks;
        max_score = max_score.max(result.score);
        if result.status == Status::Won {
            won_games += 1;
            won_ticks += result.ticks;
        }
    }
    Summary {
        games,
        avg_score: total_score as f64 / games as f64,
        avg_ticks: total_ticks as f64 / games as f64,
        max_score,
        won_frac: won_games as f64 / games as f64,
        avg_ticks_won: (won_games > 0).then(|| won_ticks as f64 / won_games as f64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::BoundaryMode;
    use crate::strategy::{ChaosWalker, Greedy, PathPlanner};

    /// The benchmark must reproduce the expected strength ordering:
    /// chaos < greedy < path planner.
    #[test]
    fn strategy_strength_ordering() {
        let base = Config {
            width: 16,
            height: 12,
            boundary: BoundaryMode::Walls,
            seed: 0,
        };
        let games = 8;
        let max_ticks = 3_000;

        let chaos = run_series(
            &mut |seed| Box::new(ChaosWalker::new(seed)),
            base,
            games,
            max_ticks,
        );
        let greedy = run_series(&mut |_| Box::new(Greedy), base, games, max_ticks);
        let planner = run_series(
            &mut |_| Box::new(PathPlanner::new()),
            base,
            games,
            max_ticks,
        );

        assert!(
            chaos.avg_score < greedy.avg_score,
            "chaos ({:.2}) should score below greedy ({:.2})",
            chaos.avg_score,
            greedy.avg_score
        );
        assert!(
            greedy.avg_score < planner.avg_score,
            "greedy ({:.2}) should score below planner ({:.2})",
            greedy.avg_score,
            planner.avg_score
        );
    }

    #[test]
    fn space_keeper_outlives_greedy_and_actually_eats() {
        use crate::strategy::SpaceKeeper;
        let base = Config {
            width: 16,
            height: 12,
            boundary: BoundaryMode::Walls,
            seed: 0,
        };
        let games = 6;
        let max_ticks = 3_000;
        let chaos = run_series(
            &mut |seed| Box::new(ChaosWalker::new(seed)),
            base,
            games,
            max_ticks,
        );
        let greedy = run_series(&mut |_| Box::new(Greedy), base, games, max_ticks);
        let space = run_series(
            &mut |_| Box::new(SpaceKeeper::new()),
            base,
            games,
            max_ticks,
        );
        // The defensive identity: far longer-lived than the pure food
        // chaser …
        assert!(
            space.avg_ticks > 2.0 * greedy.avg_ticks,
            "space keeper ({:.0} ticks) should clearly outlive greedy ({:.0} ticks)",
            space.avg_ticks,
            greedy.avg_ticks
        );
        // … while still eating. Regression guard: with a strict space
        // comparison (eating always costs one cell) or a tiebreak against
        // the respawned food, the space keeper starves at score 0.
        assert!(
            space.avg_score > chaos.avg_score,
            "space keeper ({:.2}) must actually eat, more than chaos ({:.2})",
            space.avg_score,
            chaos.avg_score
        );
    }

    #[test]
    fn ordering_holds_on_torus_too() {
        let base = Config {
            width: 16,
            height: 12,
            boundary: BoundaryMode::Periodic,
            seed: 0,
        };
        let games = 4;
        let max_ticks = 2_000;
        let chaos = run_series(
            &mut |seed| Box::new(ChaosWalker::new(seed)),
            base,
            games,
            max_ticks,
        );
        let planner = run_series(
            &mut |_| Box::new(PathPlanner::new()),
            base,
            games,
            max_ticks,
        );
        assert!(
            chaos.avg_score < planner.avg_score,
            "chaos ({:.2}) should score below planner ({:.2}) on the torus",
            chaos.avg_score,
            planner.avg_score
        );
    }
}
