//! GA/ES training: a population of flat MLP genomes, evaluated in
//! parallel with rayon, truncation selection plus Gaussian mutation.

use std::io::Write as _;
use std::path::PathBuf;

use rand::{Rng as _, SeedableRng as _};
use rand_pcg::Pcg64;
use rayon::prelude::*;

use snake_core::benchmark::play_game;
use snake_core::nn::{default_dims, Mlp, NeuralNet};
use snake_core::{BoundaryMode, Config};

#[derive(Debug, Clone)]
pub struct TrainConfig {
    pub generations: u32,
    pub population: usize,
    /// Games per genome per generation (same seeds for the whole
    /// population, rotated each generation).
    pub games_per_eval: u64,
    pub max_ticks: u64,
    /// Mutation strength.
    pub sigma: f32,
    /// Fraction of the population kept as parents.
    pub elite_frac: f64,
    pub seed: u64,
    pub board: Config,
    pub checkpoint_every: u32,
    pub out_dir: PathBuf,
}

impl Default for TrainConfig {
    fn default() -> Self {
        Self {
            generations: 300,
            population: 96,
            games_per_eval: 6,
            max_ticks: 2_000,
            sigma: 0.08,
            elite_frac: 0.25,
            seed: 1,
            board: Config {
                width: 16,
                height: 12,
                boundary: BoundaryMode::Walls,
                seed: 0,
            },
            checkpoint_every: 25,
            out_dir: PathBuf::from("training-out"),
        }
    }
}

/// Score dominates; surviving ticks break ties between equally hungry
/// genomes early in training.
fn fitness(config: &TrainConfig, genome: &[f32], eval_seeds: &[u64]) -> f64 {
    let dims = default_dims();
    let mlp = Mlp::from_params(&dims, genome.to_vec()).expect("genome has the right size");
    let mut total = 0.0;
    for &seed in eval_seeds {
        let mut strategy = NeuralNet::new(mlp.clone());
        let result = play_game(
            &mut strategy,
            Config {
                seed,
                ..config.board
            },
            config.max_ticks,
        );
        total += f64::from(result.score) * 100.0 + result.ticks as f64 * 0.1;
    }
    total / eval_seeds.len() as f64
}

/// Box–Muller: keeps the dependency list free of rand_distr.
fn gaussian(rng: &mut Pcg64) -> f32 {
    let u1: f64 = rng.random::<f64>().max(1e-12);
    let u2: f64 = rng.random();
    ((-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()) as f32
}

pub struct TrainResult {
    pub best_fitness: f64,
    pub best_path: PathBuf,
}

pub fn train(config: &TrainConfig) -> TrainResult {
    let dims = default_dims();
    let n_params = Mlp::param_count(&dims);
    let mut rng = Pcg64::seed_from_u64(config.seed);

    std::fs::create_dir_all(&config.out_dir).expect("create out dir");
    let log_path = config.out_dir.join("fitness.csv");
    let mut log = std::fs::File::create(&log_path).expect("create fitness log");
    writeln!(log, "generation,best,mean").unwrap();

    let mut population: Vec<Vec<f32>> = (0..config.population)
        .map(|_| (0..n_params).map(|_| gaussian(&mut rng) * 0.5).collect())
        .collect();

    let elites = ((config.population as f64 * config.elite_frac) as usize).max(2);
    let mut best_overall = f64::MIN;
    let best_path = config.out_dir.join("best.mlp");

    for generation in 0..config.generations {
        let eval_seeds: Vec<u64> = (0..config.games_per_eval)
            .map(|g| u64::from(generation) * 10_000 + g)
            .collect();

        let mut scored: Vec<(f64, usize)> = population
            .par_iter()
            .enumerate()
            .map(|(i, genome)| (fitness(config, genome, &eval_seeds), i))
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));

        let best = scored[0].0;
        let mean = scored.iter().map(|(f, _)| f).sum::<f64>() / scored.len() as f64;
        writeln!(log, "{generation},{best:.2},{mean:.2}").unwrap();
        println!("gen {generation:4}  best {best:9.2}  mean {mean:9.2}");

        if best > best_overall {
            best_overall = best;
            let mlp = Mlp::from_params(&dims, population[scored[0].1].clone()).unwrap();
            std::fs::write(&best_path, mlp.to_text()).expect("write best.mlp");
        }
        if config.checkpoint_every > 0 && generation % config.checkpoint_every == 0 {
            let mlp = Mlp::from_params(&dims, population[scored[0].1].clone()).unwrap();
            let path = config.out_dir.join(format!("gen_{generation:05}.mlp"));
            std::fs::write(path, mlp.to_text()).expect("write checkpoint");
        }

        // Truncation selection + Gaussian mutation.
        let parents: Vec<Vec<f32>> = scored[..elites]
            .iter()
            .map(|(_, i)| population[*i].clone())
            .collect();
        population = (0..config.population)
            .map(|i| {
                if i < elites {
                    parents[i].clone() // elites survive unchanged
                } else {
                    let parent = &parents[rng.random_range(0..parents.len())];
                    parent
                        .iter()
                        .map(|p| p + config.sigma * gaussian(&mut rng))
                        .collect()
                }
            })
            .collect();
    }

    println!("best fitness: {best_overall:.2} -> {}", best_path.display());
    TrainResult {
        best_fitness: best_overall,
        best_path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end pipeline smoke test: a tiny run must produce a parsable
    /// checkpoint and finite fitness values.
    #[test]
    fn smoke_run_produces_checkpoint() {
        let dir = std::env::temp_dir().join("hexsnake-train-test");
        let _ = std::fs::remove_dir_all(&dir);
        let config = TrainConfig {
            generations: 2,
            population: 8,
            games_per_eval: 1,
            max_ticks: 200,
            out_dir: dir.clone(),
            ..TrainConfig::default()
        };
        let result = train(&config);
        assert!(result.best_fitness.is_finite());
        let text = std::fs::read_to_string(result.best_path).expect("best.mlp written");
        let mlp = Mlp::from_text(&text).expect("checkpoint parses");
        assert_eq!(mlp.dims(), default_dims());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
