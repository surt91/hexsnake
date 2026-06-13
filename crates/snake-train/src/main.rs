//! Evolution-strategy trainer for the HexSnake neural net (native only).
//!
//! The real, long training runs are executed by the user on a beefier
//! machine — see `docs/training/neural-net-ga.md` for the full guide.
//! `cargo run --release -p snake-train -- --smoke` verifies the pipeline.

use std::path::PathBuf;

use snake_train::neat::{self, NeatConfig};
use snake_train::{train, TrainConfig};

fn main() {
    let mut raw = std::env::args().skip(1).peekable();
    // `snake-train neat ...` selects the NEAT trainer; everything else stays
    // the GA/ES trainer for backwards compatibility.
    if raw.peek().map(String::as_str) == Some("neat") {
        raw.next();
        run_neat(raw.collect());
        return;
    }

    let mut config = TrainConfig::default();
    let mut args = raw;
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .unwrap_or_else(|| panic!("missing value for {name}"))
        };
        match arg.as_str() {
            "--generations" => config.generations = value("--generations").parse().unwrap(),
            "--population" => config.population = value("--population").parse().unwrap(),
            "--games" => config.games_per_eval = value("--games").parse().unwrap(),
            "--max-ticks" => config.max_ticks = value("--max-ticks").parse().unwrap(),
            "--sigma" => config.sigma = value("--sigma").parse().unwrap(),
            "--seed" => config.seed = value("--seed").parse().unwrap(),
            "--out" => config.out_dir = PathBuf::from(value("--out")),
            "--checkpoint-every" => {
                config.checkpoint_every = value("--checkpoint-every").parse().unwrap()
            }
            "--mixed" => config.mixed_boundary = true,
            "--smoke" => {
                // Minimal end-to-end pipeline check, finishes in seconds.
                config.generations = 3;
                config.population = 12;
                config.games_per_eval = 2;
                config.max_ticks = 300;
            }
            other => {
                eprintln!("unknown argument: {other}");
                eprintln!(
                    "usage: snake-train [--smoke] [--mixed] [--generations N] [--population N] \
                     [--games N] [--max-ticks N] [--sigma F] [--seed N] \
                     [--checkpoint-every N] [--out DIR]\n   or: snake-train neat [...]"
                );
                std::process::exit(2);
            }
        }
    }
    train(&config);
}

fn run_neat(args: Vec<String>) {
    let mut config = NeatConfig::default();
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .unwrap_or_else(|| panic!("missing value for {name}"))
        };
        match arg.as_str() {
            "--generations" => config.generations = value("--generations").parse().unwrap(),
            "--population" => config.population = value("--population").parse().unwrap(),
            "--games" => config.games_per_eval = value("--games").parse().unwrap(),
            "--max-ticks" => config.max_ticks = value("--max-ticks").parse().unwrap(),
            "--seed" => config.seed = value("--seed").parse().unwrap(),
            "--out" => config.out_dir = PathBuf::from(value("--out")),
            "--checkpoint-every" => {
                config.checkpoint_every = value("--checkpoint-every").parse().unwrap()
            }
            "--compat" => config.compat_threshold = value("--compat").parse().unwrap(),
            "--add-conn" => config.add_conn_rate = value("--add-conn").parse().unwrap(),
            "--add-node" => config.add_node_rate = value("--add-node").parse().unwrap(),
            "--mixed" => config.mixed_boundary = true,
            "--smoke" => {
                config.generations = 3;
                config.population = 16;
                config.games_per_eval = 2;
                config.max_ticks = 300;
            }
            other => {
                eprintln!("unknown argument: {other}");
                eprintln!(
                    "usage: snake-train neat [--smoke] [--mixed] [--generations N] \
                     [--population N] [--games N] [--max-ticks N] [--seed N] \
                     [--compat F] [--add-conn F] [--add-node F] [--checkpoint-every N] [--out DIR]"
                );
                std::process::exit(2);
            }
        }
    }
    neat::train(&config);
}
