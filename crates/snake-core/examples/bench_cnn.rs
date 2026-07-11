//! Benchmark a specific .cnn file on Walls and Periodic topologies.
//!
//! Usage: cargo run --release -p snake-core --example bench_cnn <path.cnn> [games] [max_ticks]
//!
//! The head's output dimension decides how the net is played: a 6-output net
//! is a plain [`ConvNet`] (policy argmax over safe moves); a 7-output net is an
//! [`AlphaZeroConv`] policy/value net driven by MCTS (sims as in `embedded()`).

use snake_core::benchmark::run_series;
use snake_core::nn::{ConvNet, HexConv};
use snake_core::strategy::{AlphaZeroConv, Strategy};
use snake_core::{BoundaryMode, Config};

fn main() {
    let mut args = std::env::args().skip(1);
    let cnn_path = args
        .next()
        .expect("usage: bench_cnn <path.cnn> [games] [max_ticks]");
    let games: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(50);
    let max_ticks: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(20_000);

    let text = std::fs::read_to_string(&cnn_path).expect("cannot read cnn file");
    let net = HexConv::from_text(&text).expect("invalid cnn format");
    let out_dim = net.output_dim();

    // Sims mirror AlphaZeroConv::embedded() so 7-output nets bench as deployed.
    const AZ_SIMS: u32 = 24;
    type MakeStrategy = Box<dyn Fn() -> Box<dyn Strategy>>;
    let (mode, make): (&str, MakeStrategy) = match out_dim {
        6 => (
            "ConvNet (argmax)",
            Box::new({
                let net = net.clone();
                move || Box::new(ConvNet::new(net.clone()))
            }),
        ),
        7 => (
            "AlphaZeroConv (MCTS, sims=24)",
            Box::new({
                let net = net.clone();
                move || Box::new(AlphaZeroConv::new(net.clone(), AZ_SIMS))
            }),
        ),
        d => panic!("unexpected head output dim {d} (want 6 = ConvNet, 7 = AlphaZeroConv)"),
    };

    println!("cnn: {cnn_path}  mode={mode}  games={games}  max_ticks={max_ticks}");

    for boundary in [BoundaryMode::Walls, BoundaryMode::Periodic] {
        let base = Config {
            width: 16,
            height: 12,
            boundary,
            seed: 0,
        };
        let summary = run_series(&mut |_seed| make(), base, games, max_ticks);
        let name = match boundary {
            BoundaryMode::Walls => "Walls",
            BoundaryMode::Periodic => "Periodic",
        };
        let ticks_won = summary
            .avg_ticks_won
            .map(|t| format!("{t:.0}"))
            .unwrap_or_else(|| "—".to_string());
        println!(
            "{name}: avg_score={:.2}  avg_ticks={:.0}  max={}  won={:.1}%  ⌀ticks(won)={ticks_won}",
            summary.avg_score,
            summary.avg_ticks,
            summary.max_score,
            summary.won_frac * 100.0
        );
    }
}
