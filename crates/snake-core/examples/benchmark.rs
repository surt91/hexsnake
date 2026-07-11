//! Compare all autopilot strategies headless.
//!
//! Usage: cargo run --release -p snake-core --example benchmark [games] [max_ticks]

use snake_core::benchmark::run_series;
use snake_core::nn::{ConvNet, NeatNet, NeuralNet};
use snake_core::strategy::{
    AlphaZeroConv, AlphaZeroLite, ChaosWalker, Greedy, HamiltonRider, MonteCarlo, PathPlanner,
    SpaceKeeper, Strategy,
};
use snake_core::{Board, BoundaryMode, Config};

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
        ("Raumgreifer", Box::new(|_| Box::new(SpaceKeeper::new()))),
        (
            "Hamilton",
            Box::new(|_| {
                Box::new(
                    HamiltonRider::new(&Board::new(16, 12, BoundaryMode::Walls))
                        .expect("16x12 is hamilton-compatible"),
                )
            }),
        ),
        (
            "Monte-Carlo",
            Box::new(|seed| Box::new(MonteCarlo::new(seed))),
        ),
        ("Neural Net", Box::new(|_| Box::new(NeuralNet::embedded()))),
        ("NEAT", Box::new(|_| Box::new(NeatNet::embedded()))),
        ("DQN", Box::new(|_| Box::new(NeuralNet::embedded_dqn()))),
        ("PPO", Box::new(|_| Box::new(NeuralNet::embedded_ppo()))),
        (
            "AlphaZero-light",
            Box::new(|_| Box::new(AlphaZeroLite::embedded())),
        ),
        ("Conv-Netz", Box::new(|_| Box::new(ConvNet::embedded()))),
        (
            "AlphaZero-Conv",
            Box::new(|_| Box::new(AlphaZeroConv::embedded())),
        ),
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
            "{:<16} {:>10} {:>12} {:>10} {:>7} {:>12}",
            "Strategie", "⌀ Score", "⌀ Ticks", "max Score", "won%", "⌀ticks(won)"
        );
        for (name, make) in &mut strategies.iter_mut().map(|(n, f)| (*n, f)) {
            let summary = run_series(make.as_mut(), base, games, max_ticks);
            let ticks_won = summary
                .avg_ticks_won
                .map(|t| format!("{t:.1}"))
                .unwrap_or_else(|| "—".to_string());
            println!(
                "{:<16} {:>10.2} {:>12.1} {:>10} {:>6.1}% {:>12}",
                name,
                summary.avg_score,
                summary.avg_ticks,
                summary.max_score,
                summary.won_frac * 100.0,
                ticks_won
            );
        }
        println!();
    }
}
