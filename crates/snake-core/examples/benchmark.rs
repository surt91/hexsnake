//! Compare all autopilot strategies headless.
//!
//! Usage: cargo run --release -p snake-core --example benchmark [games] [max_ticks]

use snake_core::benchmark::run_series;
use snake_core::strategy::{ChaosWalker, Greedy, PathPlanner, Strategy};
use snake_core::{BoundaryMode, Config};

type StrategyFactory = Box<dyn FnMut(u64) -> Box<dyn Strategy>>;

fn main() {
    let mut args = std::env::args().skip(1);
    let games: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(50);
    let max_ticks: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(10_000);

    let mut strategies: Vec<(&'static str, StrategyFactory)> = vec![
        (
            "Chaos-Walker",
            Box::new(|seed| Box::new(ChaosWalker::new(seed))),
        ),
        ("Greedy", Box::new(|_| Box::new(Greedy))),
        ("Pfadplaner", Box::new(|_| Box::new(PathPlanner::new()))),
    ];

    println!("{games} Partien je Strategie, max. {max_ticks} Ticks, Feld 16×12\n");
    for boundary in [BoundaryMode::Walls, BoundaryMode::Periodic] {
        let base = Config {
            width: 16,
            height: 12,
            boundary,
            seed: 0,
        };
        println!("== {boundary:?} ==");
        println!(
            "{:<14} {:>10} {:>12} {:>10}",
            "Strategie", "⌀ Score", "⌀ Ticks", "max Score"
        );
        for (name, make) in &mut strategies.iter_mut().map(|(n, f)| (*n, f)) {
            let summary = run_series(make.as_mut(), base, games, max_ticks);
            println!(
                "{:<14} {:>10.2} {:>12.1} {:>10}",
                name, summary.avg_score, summary.avg_ticks, summary.max_score
            );
        }
        println!();
    }
}
