//! NEAT genomes: an evolvable feed-forward network whose *topology* grows
//! during training (added nodes and connections), unlike the fixed-shape
//! [`Mlp`](super::Mlp).
//!
//! `snake-core` only needs to *evaluate* a genome (pure-Rust inference, WASM
//! safe) and (de)serialize it; the evolution itself lives in `snake-train`.
//! A genome is compiled into a [`Net`] once and then evaluated every tick.
//!
//! # Genome text format (`.neat`, UTF-8)
//!
//! ```text
//! hexsnake-neat v1
//! <inputs> <outputs>
//! node <id> <I|B|H|O>
//! ...
//! conn <innovation> <from> <to> <weight> <0|1 enabled>
//! ...
//! ```
//!
//! Node ids are arbitrary but stable; by convention ids `0..inputs` are the
//! sensor inputs, the next id is the constant bias node, then the outputs,
//! then hidden nodes appended as they are created.

use std::collections::BTreeMap;

/// Role of a node in the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Input,
    Bias,
    Hidden,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGene {
    pub id: u32,
    pub kind: NodeKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConnGene {
    pub innovation: u32,
    pub from: u32,
    pub to: u32,
    pub weight: f32,
    pub enabled: bool,
}

/// A NEAT genome: nodes plus weighted, possibly disabled connections.
#[derive(Debug, Clone, PartialEq)]
pub struct Genome {
    pub inputs: usize,
    pub outputs: usize,
    pub nodes: Vec<NodeGene>,
    pub conns: Vec<ConnGene>,
}

const MAGIC: &str = "hexsnake-neat v1";

impl Genome {
    /// A minimal genome: `inputs` input nodes, one bias node and `outputs`
    /// output nodes, fully connected input+bias → outputs with the given
    /// weights generator. No hidden nodes yet — those are grown by mutation.
    pub fn minimal(inputs: usize, outputs: usize, mut weight: impl FnMut() -> f32) -> Self {
        let mut nodes = Vec::with_capacity(inputs + 1 + outputs);
        for id in 0..inputs as u32 {
            nodes.push(NodeGene {
                id,
                kind: NodeKind::Input,
            });
        }
        let bias_id = inputs as u32;
        nodes.push(NodeGene {
            id: bias_id,
            kind: NodeKind::Bias,
        });
        let first_output = inputs as u32 + 1;
        for o in 0..outputs as u32 {
            nodes.push(NodeGene {
                id: first_output + o,
                kind: NodeKind::Output,
            });
        }

        let mut conns = Vec::new();
        let mut innovation = 0;
        // Sources are the inputs and the bias node; targets are the outputs.
        for src in (0..inputs as u32).chain(std::iter::once(bias_id)) {
            for o in 0..outputs as u32 {
                conns.push(ConnGene {
                    innovation,
                    from: src,
                    to: first_output + o,
                    weight: weight(),
                    enabled: true,
                });
                innovation += 1;
            }
        }

        Self {
            inputs,
            outputs,
            nodes,
            conns,
        }
    }

    /// Id of the bias node (by the layout convention above).
    pub fn bias_id(&self) -> u32 {
        self.inputs as u32
    }

    /// Compile into a fast evaluator. Panics if the enabled connections are
    /// not feed-forward (the trainer guarantees they are).
    pub fn compile(&self) -> Net {
        // Compact node ids to indices in a deterministic (sorted) order.
        let mut index: BTreeMap<u32, usize> = BTreeMap::new();
        for (i, node) in self.nodes.iter().enumerate() {
            index.insert(node.id, i);
        }
        let n = self.nodes.len();
        let kinds: Vec<NodeKind> = self.nodes.iter().map(|nd| nd.kind).collect();

        let mut incoming: Vec<Vec<(usize, f32)>> = vec![Vec::new(); n];
        let mut in_degree = vec![0usize; n];
        for c in &self.conns {
            if !c.enabled {
                continue;
            }
            let (Some(&from), Some(&to)) = (index.get(&c.from), index.get(&c.to)) else {
                continue;
            };
            incoming[to].push((from, c.weight));
            in_degree[to] += 1;
        }

        // Kahn's algorithm → a topological evaluation order.
        let mut order = Vec::with_capacity(n);
        let mut queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
        let mut head = 0;
        while head < queue.len() {
            let node = queue[head];
            head += 1;
            order.push(node);
            // Relax outgoing edges. Rebuild from conns for the node's id.
            let node_id = self.nodes[node].id;
            for c in &self.conns {
                if c.enabled && c.from == node_id {
                    if let Some(&to) = index.get(&c.to) {
                        in_degree[to] -= 1;
                        if in_degree[to] == 0 {
                            queue.push(to);
                        }
                    }
                }
            }
        }
        assert_eq!(order.len(), n, "genome connections are not feed-forward");

        let input_idx: Vec<usize> = (0..self.inputs as u32).map(|id| index[&id]).collect();
        let bias_idx = index.get(&self.bias_id()).copied();
        let output_idx: Vec<usize> = self
            .nodes
            .iter()
            .filter(|nd| nd.kind == NodeKind::Output)
            .map(|nd| index[&nd.id])
            .collect();

        Net {
            kinds,
            incoming,
            order,
            input_idx,
            bias_idx,
            output_idx,
        }
    }

    pub fn to_text(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        out.push_str(MAGIC);
        out.push('\n');
        let _ = writeln!(out, "{} {}", self.inputs, self.outputs);
        for node in &self.nodes {
            let kind = match node.kind {
                NodeKind::Input => 'I',
                NodeKind::Bias => 'B',
                NodeKind::Hidden => 'H',
                NodeKind::Output => 'O',
            };
            let _ = writeln!(out, "node {} {kind}", node.id);
        }
        for c in &self.conns {
            // `{:?}` prints the shortest exactly round-tripping f32 form.
            let _ = writeln!(
                out,
                "conn {} {} {} {:?} {}",
                c.innovation,
                c.from,
                c.to,
                c.weight,
                u8::from(c.enabled)
            );
        }
        out
    }

    pub fn from_text(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        let magic = lines.next().ok_or("empty file")?.trim();
        if magic != MAGIC {
            return Err(format!("bad magic line: {magic:?}"));
        }
        let header = lines.next().ok_or("missing dims line")?;
        let mut dims = header.split_whitespace();
        let inputs: usize = parse_field(dims.next(), "inputs")?;
        let outputs: usize = parse_field(dims.next(), "outputs")?;

        let mut nodes = Vec::new();
        let mut conns = Vec::new();
        for line in lines {
            let mut t = line.split_whitespace();
            match t.next() {
                None => continue,
                Some("node") => {
                    let id: u32 = parse_field(t.next(), "node id")?;
                    let kind = match t.next() {
                        Some("I") => NodeKind::Input,
                        Some("B") => NodeKind::Bias,
                        Some("H") => NodeKind::Hidden,
                        Some("O") => NodeKind::Output,
                        other => return Err(format!("bad node kind: {other:?}")),
                    };
                    nodes.push(NodeGene { id, kind });
                }
                Some("conn") => {
                    let innovation: u32 = parse_field(t.next(), "innovation")?;
                    let from: u32 = parse_field(t.next(), "from")?;
                    let to: u32 = parse_field(t.next(), "to")?;
                    let weight: f32 = parse_field(t.next(), "weight")?;
                    let enabled: u8 = parse_field(t.next(), "enabled")?;
                    conns.push(ConnGene {
                        innovation,
                        from,
                        to,
                        weight,
                        enabled: enabled != 0,
                    });
                }
                Some(other) => return Err(format!("unexpected line tag: {other:?}")),
            }
        }
        if nodes.is_empty() {
            return Err("genome has no nodes".into());
        }
        Ok(Self {
            inputs,
            outputs,
            nodes,
            conns,
        })
    }
}

fn parse_field<T: std::str::FromStr>(field: Option<&str>, what: &str) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    field
        .ok_or_else(|| format!("missing {what}"))?
        .parse()
        .map_err(|e| format!("bad {what}: {e}"))
}

/// A compiled, fast-to-evaluate feed-forward network.
#[derive(Debug, Clone)]
pub struct Net {
    kinds: Vec<NodeKind>,
    incoming: Vec<Vec<(usize, f32)>>,
    order: Vec<usize>,
    input_idx: Vec<usize>,
    bias_idx: Option<usize>,
    output_idx: Vec<usize>,
}

impl Net {
    /// Evaluate the network. Hidden nodes use `tanh`, outputs are linear.
    pub fn forward(&self, features: &[f32]) -> Vec<f32> {
        let mut val = vec![0.0f32; self.kinds.len()];
        for (i, &idx) in self.input_idx.iter().enumerate() {
            val[idx] = features.get(i).copied().unwrap_or(0.0);
        }
        if let Some(b) = self.bias_idx {
            val[b] = 1.0;
        }
        for &node in &self.order {
            match self.kinds[node] {
                NodeKind::Input | NodeKind::Bias => {}
                NodeKind::Hidden | NodeKind::Output => {
                    let mut sum = 0.0;
                    for &(src, w) in &self.incoming[node] {
                        sum += val[src] * w;
                    }
                    val[node] = if self.kinds[node] == NodeKind::Output {
                        sum
                    } else {
                        sum.tanh()
                    };
                }
            }
        }
        self.output_idx.iter().map(|&i| val[i]).collect()
    }

    pub fn output_count(&self) -> usize {
        self.output_idx.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_genome_shape() {
        let g = Genome::minimal(20, 6, || 0.0);
        assert_eq!(g.nodes.len(), 20 + 1 + 6);
        assert_eq!(g.conns.len(), (20 + 1) * 6);
        let net = g.compile();
        assert_eq!(net.output_count(), 6);
    }

    #[test]
    fn forward_is_deterministic_and_sized() {
        let mut w = 0.1f32;
        let g = Genome::minimal(20, 6, || {
            w += 0.01;
            w
        });
        let net = g.compile();
        let feats = vec![0.5f32; 20];
        let a = net.forward(&feats);
        let b = net.forward(&feats);
        assert_eq!(a, b);
        assert_eq!(a.len(), 6);
    }

    #[test]
    fn bias_only_network_outputs_bias_weight() {
        // 1 input, 1 output, weights: input->out = 0, bias->out = 0.7.
        let mut genome = Genome::minimal(1, 1, || 0.0);
        // Set the bias connection weight to 0.7.
        let bias = genome.bias_id();
        for c in &mut genome.conns {
            if c.from == bias {
                c.weight = 0.7;
            }
        }
        let net = genome.compile();
        let out = net.forward(&[123.0]); // input weight is 0 ⇒ irrelevant
        assert!((out[0] - 0.7).abs() < 1e-6);
    }

    #[test]
    fn hidden_node_is_evaluated_in_order() {
        // input(0) -> hidden(8) -> output(2); bias(1) present but unused.
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
                    weight: 2.0,
                    enabled: true,
                },
            ],
        };
        let net = genome.compile();
        // out = 2.0 * tanh(1.0 * 0.5)
        let out = net.forward(&[0.5]);
        assert!((out[0] - 2.0 * 0.5f32.tanh()).abs() < 1e-6);
    }

    #[test]
    fn disabled_connection_is_ignored() {
        let mut genome = Genome::minimal(1, 1, || 1.0);
        for c in &mut genome.conns {
            c.enabled = false;
        }
        let out = genome.compile().forward(&[5.0]);
        assert_eq!(out[0], 0.0, "no enabled inputs ⇒ zero");
    }

    #[test]
    fn text_roundtrip_is_exact() {
        let mut w = -1.0f32;
        let g = Genome::minimal(4, 3, || {
            w += 0.137;
            w
        });
        let restored = Genome::from_text(&g.to_text()).unwrap();
        assert_eq!(g, restored);
    }

    #[test]
    fn rejects_garbage() {
        assert!(Genome::from_text("nope").is_err());
        assert!(Genome::from_text("hexsnake-neat v1\n").is_err());
        assert!(Genome::from_text("hexsnake-neat v1\n2 1\nnode 0 X").is_err());
    }
}
