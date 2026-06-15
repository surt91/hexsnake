//! Benchmark a specific .mlp file on Walls and Periodic topologies.
//!
//! Usage: cargo run --release -p snake-core --example bench_mlp <path.mlp> [games] [max_ticks] [sims]

use snake_core::benchmark::run_series;
use snake_core::nn::Mlp;
use snake_core::strategy::AlphaZeroLite;
use snake_core::{BoundaryMode, Config};

fn main() {
    let mut args = std::env::args().skip(1);
    let mlp_path = args
        .next()
        .expect("usage: bench_mlp <path.mlp> [games] [max_ticks] [sims]");
    let games: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(50);
    let max_ticks: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(8000);
    let sims: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(24);

    let text = std::fs::read_to_string(&mlp_path).expect("cannot read mlp file");
    let mlp = Mlp::from_text(&text).expect("invalid mlp format");

    println!("mlp: {mlp_path}  games={games}  max_ticks={max_ticks}  sims={sims}");

    for boundary in [BoundaryMode::Walls, BoundaryMode::Periodic] {
        let base = Config {
            width: 16,
            height: 12,
            boundary,
            seed: 0,
        };
        let mlp_ref = mlp.clone();
        let summary = run_series(
            &mut |_seed| Box::new(AlphaZeroLite::new(mlp_ref.clone(), sims)),
            base,
            games,
            max_ticks,
        );
        let name = match boundary {
            BoundaryMode::Walls => "Walls",
            BoundaryMode::Periodic => "Periodic",
        };
        println!(
            "{name}: avg_score={:.2}  avg_ticks={:.0}  max={}",
            summary.avg_score, summary.avg_ticks, summary.max_score
        );
    }
}
