//! NEAT training: evolve network *topology* (not just weights), with
//! innovation tracking, the three structural mutations, crossover aligned by
//! innovation number, and speciation with explicit fitness sharing.
//!
//! Inference lives in `snake-core` ([`Genome`]/`NeatNet`); this module only
//! grows and selects genomes. Fitness reuses the same headless `play_game`
//! harness as the GA/ES trainer, so scores are directly comparable.

use std::collections::HashMap;
use std::io::Write as _;
use std::path::PathBuf;

use rand::{Rng as _, SeedableRng as _};
use rand_pcg::Pcg64;
use rayon::prelude::*;

use snake_core::benchmark::play_game;
use snake_core::nn::{ConnGene, Genome, NeatNet, NodeGene, NodeKind, FEATURE_COUNT};
use snake_core::{BoundaryMode, Config};

const OUTPUTS: usize = 6;

#[derive(Debug, Clone)]
pub struct NeatConfig {
    pub generations: u32,
    pub population: usize,
    pub games_per_eval: u64,
    pub max_ticks: u64,
    pub seed: u64,
    pub board: Config,
    pub mixed_boundary: bool,
    /// Compatibility distance threshold for putting two genomes in one species.
    pub compat_threshold: f64,
    /// Coefficients for excess/disjoint/weight terms of the distance.
    pub c_excess_disjoint: f64,
    pub c_weight: f64,
    /// Top fraction of a species allowed to reproduce.
    pub survival_threshold: f64,
    /// Per-connection probability of a weight mutation, and of a full reset.
    pub weight_mut_rate: f64,
    pub weight_replace_rate: f64,
    pub weight_sigma: f32,
    /// Structural mutation rates.
    pub add_conn_rate: f64,
    pub add_node_rate: f64,
    pub checkpoint_every: u32,
    pub out_dir: PathBuf,
}

impl Default for NeatConfig {
    fn default() -> Self {
        Self {
            generations: 300,
            population: 150,
            games_per_eval: 6,
            max_ticks: 2_000,
            seed: 1,
            board: Config {
                width: 16,
                height: 12,
                boundary: BoundaryMode::Walls,
                seed: 0,
            },
            mixed_boundary: false,
            compat_threshold: 3.0,
            c_excess_disjoint: 1.0,
            c_weight: 0.4,
            survival_threshold: 0.4,
            weight_mut_rate: 0.8,
            weight_replace_rate: 0.1,
            weight_sigma: 0.1,
            add_conn_rate: 0.06,
            add_node_rate: 0.03,
            checkpoint_every: 25,
            out_dir: PathBuf::from("training-out"),
        }
    }
}

/// Tracks innovation numbers so that the same structural change gets the same
/// id across the population — the basis for aligning genes in crossover.
struct Innovations {
    conn: HashMap<(u32, u32), u32>,
    next_conn: u32,
    next_node: u32,
}

impl Innovations {
    fn from_minimal(minimal: &Genome) -> Self {
        let mut conn = HashMap::new();
        let mut next_conn = 0;
        for c in &minimal.conns {
            conn.insert((c.from, c.to), c.innovation);
            next_conn = next_conn.max(c.innovation + 1);
        }
        let next_node = minimal.nodes.iter().map(|n| n.id + 1).max().unwrap_or(0);
        Self {
            conn,
            next_conn,
            next_node,
        }
    }

    fn conn_innovation(&mut self, from: u32, to: u32) -> u32 {
        *self.conn.entry((from, to)).or_insert_with(|| {
            let i = self.next_conn;
            self.next_conn += 1;
            i
        })
    }

    fn new_node(&mut self) -> u32 {
        let id = self.next_node;
        self.next_node += 1;
        id
    }
}

fn gaussian(rng: &mut Pcg64) -> f32 {
    let u1: f64 = rng.random::<f64>().max(1e-12);
    let u2: f64 = rng.random();
    ((-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()) as f32
}

fn kind_of(genome: &Genome, id: u32) -> Option<NodeKind> {
    genome.nodes.iter().find(|n| n.id == id).map(|n| n.kind)
}

/// Would adding `from → to` create a cycle among the enabled connections?
/// True if `to` can already reach `from` (or they are equal).
fn creates_cycle(genome: &Genome, from: u32, to: u32) -> bool {
    if from == to {
        return true;
    }
    let mut stack = vec![to];
    let mut seen = vec![to];
    while let Some(node) = stack.pop() {
        if node == from {
            return true;
        }
        for c in &genome.conns {
            if c.enabled && c.from == node && !seen.contains(&c.to) {
                seen.push(c.to);
                stack.push(c.to);
            }
        }
    }
    false
}

fn mutate(genome: &mut Genome, innov: &mut Innovations, rng: &mut Pcg64, cfg: &NeatConfig) {
    // Weight mutations.
    for c in &mut genome.conns {
        if rng.random::<f64>() < cfg.weight_mut_rate {
            if rng.random::<f64>() < cfg.weight_replace_rate {
                c.weight = gaussian(rng);
            } else {
                c.weight += cfg.weight_sigma * gaussian(rng);
            }
        }
    }

    // Add connection: random valid, acyclic, non-duplicate edge.
    if rng.random::<f64>() < cfg.add_conn_rate {
        let sources: Vec<u32> = genome
            .nodes
            .iter()
            .filter(|n| n.kind != NodeKind::Output)
            .map(|n| n.id)
            .collect();
        let targets: Vec<u32> = genome
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Hidden | NodeKind::Output))
            .map(|n| n.id)
            .collect();
        if !sources.is_empty() && !targets.is_empty() {
            let from = sources[rng.random_range(0..sources.len())];
            let to = targets[rng.random_range(0..targets.len())];
            let exists = genome.conns.iter().any(|c| c.from == from && c.to == to);
            if !exists && !creates_cycle(genome, from, to) {
                let innovation = innov.conn_innovation(from, to);
                genome.conns.push(ConnGene {
                    innovation,
                    from,
                    to,
                    weight: gaussian(rng),
                    enabled: true,
                });
            }
        }
    }

    // Add node: split a random enabled connection.
    if rng.random::<f64>() < cfg.add_node_rate {
        let enabled: Vec<usize> = (0..genome.conns.len())
            .filter(|&i| genome.conns[i].enabled)
            .collect();
        if !enabled.is_empty() {
            let idx = enabled[rng.random_range(0..enabled.len())];
            let (from, to, weight) = {
                let c = &mut genome.conns[idx];
                c.enabled = false;
                (c.from, c.to, c.weight)
            };
            let new_id = innov.new_node();
            genome.nodes.push(NodeGene {
                id: new_id,
                kind: NodeKind::Hidden,
            });
            // from → new (weight 1), new → to (old weight): preserves the
            // original function right after the split.
            let i1 = innov.conn_innovation(from, new_id);
            let i2 = innov.conn_innovation(new_id, to);
            genome.conns.push(ConnGene {
                innovation: i1,
                from,
                to: new_id,
                weight: 1.0,
                enabled: true,
            });
            genome.conns.push(ConnGene {
                innovation: i2,
                from: new_id,
                to,
                weight,
                enabled: true,
            });
        }
    }
}

/// Kahn's topological-sort check on the genome's enabled connections.
/// Returns `true` iff the enabled sub-graph is acyclic (feed-forward).
fn is_feed_forward(genome: &Genome) -> bool {
    let n = genome.nodes.len();
    let index: HashMap<u32, usize> = genome
        .nodes
        .iter()
        .enumerate()
        .map(|(i, nd)| (nd.id, i))
        .collect();
    let mut in_deg = vec![0usize; n];
    for c in genome.conns.iter().filter(|c| c.enabled) {
        if let Some(&t) = index.get(&c.to) {
            in_deg[t] += 1;
        }
    }
    let mut queue: Vec<usize> = (0..n).filter(|&i| in_deg[i] == 0).collect();
    let mut head = 0;
    while head < queue.len() {
        let nd = queue[head];
        head += 1;
        let nd_id = genome.nodes[nd].id;
        for c in genome.conns.iter().filter(|c| c.enabled && c.from == nd_id) {
            if let Some(&t) = index.get(&c.to) {
                in_deg[t] -= 1;
                if in_deg[t] == 0 {
                    queue.push(t);
                }
            }
        }
    }
    queue.len() == n
}

/// Crossover aligned by innovation: matching genes are inherited at random,
/// disjoint/excess genes come from the fitter parent (`a`).
fn crossover(a: &Genome, b: &Genome, rng: &mut Pcg64) -> Genome {
    let b_by_innov: HashMap<u32, &ConnGene> = b.conns.iter().map(|c| (c.innovation, c)).collect();

    let mut conns = Vec::with_capacity(a.conns.len());
    for ca in &a.conns {
        let mut gene = *ca;
        if let Some(cb) = b_by_innov.get(&ca.innovation) {
            if rng.random::<bool>() {
                gene.weight = cb.weight;
            }
            // A gene disabled in either parent is likely (75%) disabled.
            let disabled_in_either = !ca.enabled || !cb.enabled;
            gene.enabled = !(disabled_in_either && rng.random::<f64>() < 0.75);
        }
        conns.push(gene);
    }

    // Collect every node id referenced by the inherited connections, plus the
    // fixed input/bias/output nodes from the fitter parent.
    let mut nodes: Vec<NodeGene> = a
        .nodes
        .iter()
        .filter(|n| n.kind != NodeKind::Hidden)
        .copied()
        .collect();
    for c in &conns {
        for id in [c.from, c.to] {
            if !nodes.iter().any(|n| n.id == id) {
                let kind = kind_of(a, id)
                    .or_else(|| kind_of(b, id))
                    .unwrap_or(NodeKind::Hidden);
                nodes.push(NodeGene { id, kind });
            }
        }
    }

    let child = Genome {
        inputs: a.inputs,
        outputs: a.outputs,
        nodes,
        conns,
    };
    // Crossover can re-enable previously disabled connections (25 % chance),
    // which may introduce cycles. Fall back to the fitter parent when that
    // happens so the population stays feed-forward.
    if is_feed_forward(&child) {
        child
    } else {
        a.clone()
    }
}

/// Compatibility distance for speciation.
fn distance(a: &Genome, b: &Genome, cfg: &NeatConfig) -> f64 {
    let a_by: HashMap<u32, &ConnGene> = a.conns.iter().map(|c| (c.innovation, c)).collect();
    let b_by: HashMap<u32, &ConnGene> = b.conns.iter().map(|c| (c.innovation, c)).collect();
    let mut matching = 0u32;
    let mut weight_diff = 0.0f64;
    let mut mismatched = 0u32;
    for (innov, ca) in &a_by {
        if let Some(cb) = b_by.get(innov) {
            matching += 1;
            weight_diff += (ca.weight - cb.weight).abs() as f64;
        } else {
            mismatched += 1;
        }
    }
    for innov in b_by.keys() {
        if !a_by.contains_key(innov) {
            mismatched += 1;
        }
    }
    let n = a.conns.len().max(b.conns.len()).max(1) as f64;
    let avg_w = if matching > 0 {
        weight_diff / matching as f64
    } else {
        0.0
    };
    cfg.c_excess_disjoint * mismatched as f64 / n + cfg.c_weight * avg_w
}

pub struct NeatResult {
    pub best_fitness: f64,
    pub best_path: PathBuf,
}

fn fitness(cfg: &NeatConfig, genome: &Genome, eval_seeds: &[u64]) -> f64 {
    let mut total = 0.0;
    for (i, &seed) in eval_seeds.iter().enumerate() {
        let boundary = if cfg.mixed_boundary && i % 2 == 1 {
            BoundaryMode::Periodic
        } else {
            cfg.board.boundary
        };
        let mut strategy = NeatNet::new(genome);
        let result = play_game(
            &mut strategy,
            Config {
                seed,
                boundary,
                ..cfg.board
            },
            cfg.max_ticks,
        );
        total += f64::from(result.score) * 100.0 + result.ticks as f64 * 0.1;
    }
    total / eval_seeds.len() as f64
}

pub fn train(cfg: &NeatConfig) -> NeatResult {
    let mut rng = Pcg64::seed_from_u64(cfg.seed);

    // A minimal genome fixes the input/bias/output layout and seeds the
    // innovation numbering shared by the whole run.
    let template = Genome::minimal(FEATURE_COUNT, OUTPUTS, || 0.0);
    let mut innov = Innovations::from_minimal(&template);

    let mut population: Vec<Genome> = (0..cfg.population)
        .map(|_| {
            let mut g = template.clone();
            for c in &mut g.conns {
                c.weight = gaussian(&mut rng);
            }
            g
        })
        .collect();

    std::fs::create_dir_all(&cfg.out_dir).expect("create out dir");
    let log_path = cfg.out_dir.join("fitness.csv");
    let mut log = std::fs::File::create(&log_path).expect("create fitness log");
    writeln!(log, "generation,best,mean,species").unwrap();

    let mut best_overall = f64::MIN;
    let best_path = cfg.out_dir.join("best.neat");

    for generation in 0..cfg.generations {
        let eval_seeds: Vec<u64> = (0..cfg.games_per_eval)
            .map(|g| u64::from(generation) * 10_000 + g)
            .collect();

        let scores: Vec<f64> = population
            .par_iter()
            .map(|g| fitness(cfg, g, &eval_seeds))
            .collect();

        let (best_idx, best) = scores
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, f)| (i, *f))
            .unwrap();
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;

        let species = speciate(&population, cfg);
        writeln!(log, "{generation},{best:.2},{mean:.2},{}", species.len()).unwrap();
        println!(
            "gen {generation:4}  best {best:9.2}  mean {mean:9.2}  species {:3}  nodes {:3}",
            species.len(),
            population[best_idx].nodes.len()
        );

        if best > best_overall {
            best_overall = best;
            std::fs::write(&best_path, population[best_idx].to_text()).expect("write best.neat");
        }
        if cfg.checkpoint_every > 0 && generation % cfg.checkpoint_every == 0 {
            let path = cfg.out_dir.join(format!("gen_{generation:05}.neat"));
            std::fs::write(path, population[best_idx].to_text()).expect("write checkpoint");
        }

        population = reproduce(&population, &scores, &species, &mut innov, &mut rng, cfg);
    }

    println!("best fitness: {best_overall:.2} -> {}", best_path.display());
    NeatResult {
        best_fitness: best_overall,
        best_path,
    }
}

/// Partition the population into species by compatibility distance to the
/// first member (representative) of each existing species.
fn speciate(population: &[Genome], cfg: &NeatConfig) -> Vec<Vec<usize>> {
    let mut species: Vec<Vec<usize>> = Vec::new();
    for (i, genome) in population.iter().enumerate() {
        let mut placed = false;
        for group in &mut species {
            let rep = &population[group[0]];
            if distance(genome, rep, cfg) < cfg.compat_threshold {
                group.push(i);
                placed = true;
                break;
            }
        }
        if !placed {
            species.push(vec![i]);
        }
    }
    species
}

/// Produce the next generation with fitness sharing: each species gets a
/// share of offspring proportional to its summed adjusted fitness; the best
/// member of a large species survives unchanged (elitism).
fn reproduce(
    population: &[Genome],
    scores: &[f64],
    species: &[Vec<usize>],
    innov: &mut Innovations,
    rng: &mut Pcg64,
    cfg: &NeatConfig,
) -> Vec<Genome> {
    // Shift fitness to be non-negative so the proportional split is sane.
    let min_score = scores.iter().cloned().fold(f64::INFINITY, f64::min);
    let shift = if min_score < 0.0 { -min_score } else { 0.0 };

    // Adjusted (shared) fitness sum per species.
    let adjusted: Vec<f64> = species
        .iter()
        .map(|group| {
            group
                .iter()
                .map(|&i| (scores[i] + shift) / group.len() as f64)
                .sum::<f64>()
        })
        .collect();
    let total: f64 = adjusted.iter().sum::<f64>().max(1e-9);

    let mut next: Vec<Genome> = Vec::with_capacity(cfg.population);
    for (group, &adj) in species.iter().zip(&adjusted) {
        let offspring = ((adj / total) * cfg.population as f64).round() as usize;
        if offspring == 0 {
            continue;
        }

        // Survivors: top `survival_threshold` of the species by fitness.
        let mut ranked = group.clone();
        ranked.sort_by(|&x, &y| scores[y].total_cmp(&scores[x]));
        let keep = ((ranked.len() as f64 * cfg.survival_threshold).ceil() as usize).max(1);
        let survivors = &ranked[..keep];

        // Elitism: the species champion carries over unchanged.
        next.push(population[ranked[0]].clone());

        for _ in 1..offspring {
            if next.len() >= cfg.population {
                break;
            }
            let p1 = survivors[rng.random_range(0..survivors.len())];
            let p2 = survivors[rng.random_range(0..survivors.len())];
            // Crossover orders parents fitter-first.
            let mut child = if scores[p1] >= scores[p2] {
                crossover(&population[p1], &population[p2], rng)
            } else {
                crossover(&population[p2], &population[p1], rng)
            };
            mutate(&mut child, innov, rng, cfg);
            next.push(child);
        }
    }

    // Top up / trim to the exact population size.
    while next.len() < cfg.population {
        let mut clone = population[0].clone();
        mutate(&mut clone, innov, rng, cfg);
        next.push(clone);
    }
    next.truncate(cfg.population);
    next
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_detection_blocks_back_edges() {
        // 0(input) -> 8(hidden) -> 2(output); adding 2 -> 8 would cycle.
        let genome = Genome {
            inputs: 1,
            outputs: 1,
            nodes: vec![
                NodeGene {
                    id: 0,
                    kind: NodeKind::Input,
                },
                NodeGene {
                    id: 1,
                    kind: NodeKind::Bias,
                },
                NodeGene {
                    id: 2,
                    kind: NodeKind::Output,
                },
                NodeGene {
                    id: 8,
                    kind: NodeKind::Hidden,
                },
            ],
            conns: vec![
                ConnGene {
                    innovation: 0,
                    from: 0,
                    to: 8,
                    weight: 1.0,
                    enabled: true,
                },
                ConnGene {
                    innovation: 1,
                    from: 8,
                    to: 2,
                    weight: 1.0,
                    enabled: true,
                },
            ],
        };
        assert!(creates_cycle(&genome, 2, 8), "back edge must be rejected");
        assert!(!creates_cycle(&genome, 0, 2), "forward edge is fine");
    }

    #[test]
    fn mutations_keep_genome_compilable() {
        let mut rng = Pcg64::seed_from_u64(7);
        let template = Genome::minimal(FEATURE_COUNT, OUTPUTS, || 0.0);
        let mut innov = Innovations::from_minimal(&template);
        let mut g = template.clone();
        let cfg = NeatConfig {
            add_conn_rate: 1.0,
            add_node_rate: 1.0,
            ..NeatConfig::default()
        };
        for _ in 0..200 {
            mutate(&mut g, &mut innov, &mut rng, &cfg);
            // Compiling asserts the structure stayed feed-forward.
            let net = g.compile();
            assert_eq!(net.output_count(), OUTPUTS);
        }
        assert!(
            g.nodes.len() > template.nodes.len(),
            "add-node mutations should have grown the network"
        );
    }

    #[test]
    fn smoke_run_produces_parsable_checkpoint() {
        let dir = std::env::temp_dir().join("hexsnake-neat-test");
        let _ = std::fs::remove_dir_all(&dir);
        let cfg = NeatConfig {
            generations: 2,
            population: 12,
            games_per_eval: 1,
            max_ticks: 200,
            out_dir: dir.clone(),
            ..NeatConfig::default()
        };
        let result = train(&cfg);
        assert!(result.best_fitness.is_finite());
        let text = std::fs::read_to_string(&result.best_path).expect("best.neat written");
        let genome = Genome::from_text(&text).expect("checkpoint parses");
        assert_eq!(genome.inputs, FEATURE_COUNT);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
