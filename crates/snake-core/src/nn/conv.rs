//! Hex-convolutional inference: a pure-Rust forward pass that sees the whole
//! board as a grid tensor instead of the six sensor rays.
//!
//! The board is a hex grid, so the kernel is **7-tap** (center + 6 neighbors)
//! rather than a square 3×3. Neighbor gathering goes through [`Board::neighbor`],
//! which already handles walls (off-board → zero-pad) and the periodic torus
//! (wrap) correctly — so the convolution is deterministic and torus-correct
//! with no special cases.
//!
//! Size-independence without going position-blind: after the conv stack the
//! feature vector **at the head cell** (local context) is concatenated with a
//! **global average pool** (board-wide summary) and fed to a dense [`Mlp`] head.
//! Both halves are independent of board width/height, so one net plays on any
//! size.
//!
//! # Weight format (`.cnn`, UTF-8 text)
//!
//! ```text
//! hexsnake-cnn v1          <- magic + version
//! channels 4               <- number of input planes
//! conv 4 8                 <- conv layer: in_ch out_ch (kernel is implicitly 7-tap)
//! conv 8 8
//! head 16 12 6             <- dense head dims (readout⊕pool → … → outputs)
//! params                   <- everything after this line is parameters
//! 0.1 -0.5 …               <- per conv layer: weights (out×in×7, row-major)
//!                             then biases; then the head's Mlp params
//! ```
//!
//! Conv layers use tanh; the head's output layer is linear (like [`Mlp`]).

use crate::board::{Board, BoundaryMode};
use crate::coords::{Direction, Offset};
use crate::game::GameState;
use crate::nn::Mlp;

/// Input planes: body (excl. head), head, food, topology (1 = walls, 0 = torus).
pub const CONV_CHANNELS: usize = 4;
/// Kernel taps: the cell itself plus its six hex neighbors.
const TAPS: usize = 7;
const MAGIC: &str = "hexsnake-cnn v1";

/// One hex-convolution layer.
#[derive(Debug, Clone, PartialEq)]
struct ConvLayer {
    in_ch: usize,
    out_ch: usize,
    /// `out_ch × in_ch × 7`, row-major (`[o][i][tap]`). Tap 0 = center,
    /// taps 1..6 = `Direction::ALL` in order.
    weights: Vec<f32>,
    /// One per output channel.
    biases: Vec<f32>,
}

impl ConvLayer {
    fn param_count(in_ch: usize, out_ch: usize) -> usize {
        out_ch * in_ch * TAPS + out_ch
    }
}

/// A hex-conv net: a stack of conv layers followed by a dense [`Mlp`] head fed
/// from the head-cell readout concatenated with the global average pool.
#[derive(Debug, Clone, PartialEq)]
pub struct HexConv {
    in_channels: usize,
    conv_layers: Vec<ConvLayer>,
    head: Mlp,
}

impl HexConv {
    /// Build from layer shapes, head dims and a flat parameter vector (conv
    /// layers first — weights then biases each — then the head's params).
    pub fn from_params(
        in_channels: usize,
        conv_shapes: &[(usize, usize)],
        head_dims: &[usize],
        params: Vec<f32>,
    ) -> Result<Self, String> {
        // Channel chain must be consistent: first conv reads the input planes,
        // each next conv reads the previous output.
        let mut expect_in = in_channels;
        for &(i, _o) in conv_shapes {
            if i != expect_in {
                return Err(format!(
                    "conv in_ch {i} does not match previous out_ch {expect_in}"
                ));
            }
            expect_in = _o;
        }
        let last_out = conv_shapes.last().map(|&(_, o)| o).unwrap_or(in_channels);
        // Head input = head-cell readout ⊕ global pool = 2 × last conv channels.
        if head_dims.first() != Some(&(2 * last_out)) {
            return Err(format!(
                "head input dim {:?} must be 2×last_conv_out_ch ({})",
                head_dims.first(),
                2 * last_out
            ));
        }

        let conv_params: usize = conv_shapes
            .iter()
            .map(|&(i, o)| ConvLayer::param_count(i, o))
            .sum();
        let head_params = Mlp::param_count(head_dims);
        let expected = conv_params + head_params;
        if params.len() != expected {
            return Err(format!(
                "expected {expected} params ({conv_params} conv + {head_params} head), got {}",
                params.len()
            ));
        }

        let mut off = 0;
        let mut conv_layers = Vec::with_capacity(conv_shapes.len());
        for &(in_ch, out_ch) in conv_shapes {
            let n_w = out_ch * in_ch * TAPS;
            let weights = params[off..off + n_w].to_vec();
            let biases = params[off + n_w..off + n_w + out_ch].to_vec();
            off += n_w + out_ch;
            conv_layers.push(ConvLayer {
                in_ch,
                out_ch,
                weights,
                biases,
            });
        }
        let head = Mlp::from_params(head_dims, params[off..].to_vec())?;

        Ok(Self {
            in_channels,
            conv_layers,
            head,
        })
    }

    /// Output dimension (the head's last layer).
    pub fn output_dim(&self) -> usize {
        *self.head.dims().last().expect("head has dims")
    }

    /// Forward pass on a game state: build the input planes, run the conv stack,
    /// then head-cell readout ⊕ global pool → head.
    pub fn forward(&self, state: &GameState) -> Vec<f32> {
        let board = state.board();
        let (planes, w, h) = build_planes(state);
        let head = state.head();
        let head_i = (head.row as usize) * w + head.col as usize;
        self.forward_planes(&planes, w, h, board.boundary, head_i)
    }

    /// Lower-level forward over explicit planes (`in_channels × h × w`,
    /// channel-major then row-major). Kept separate so tests and the Python
    /// roundtrip check can drive known inputs without a `GameState`.
    pub fn forward_planes(
        &self,
        planes: &[f32],
        w: usize,
        h: usize,
        boundary: BoundaryMode,
        head_i: usize,
    ) -> Vec<f32> {
        assert_eq!(
            planes.len(),
            self.in_channels * w * h,
            "plane buffer does not match in_channels × w × h"
        );
        let board = Board::new(w as i32, h as i32, boundary);
        let plane = w * h;

        let mut cur = planes.to_vec();
        let mut in_ch = self.in_channels;
        for layer in &self.conv_layers {
            let mut next = vec![0.0f32; layer.out_ch * plane];
            for row in 0..h {
                for col in 0..w {
                    let cell = Offset::new(col as i32, row as i32);
                    let cell_i = row * w + col;
                    // Source index per tap: center, then the six neighbors.
                    // Off-board neighbors stay `None` and contribute zero.
                    let mut taps = [None; TAPS];
                    taps[0] = Some(cell_i);
                    for (t, dir) in Direction::ALL.iter().enumerate() {
                        taps[t + 1] = board
                            .neighbor(cell, *dir)
                            .map(|n| (n.row as usize) * w + n.col as usize);
                    }
                    for o in 0..layer.out_ch {
                        let mut sum = layer.biases[o];
                        let w_o = o * in_ch * TAPS;
                        for ci in 0..in_ch {
                            let src = &cur[ci * plane..ci * plane + plane];
                            let w_oi = w_o + ci * TAPS;
                            for (t, &tap) in taps.iter().enumerate() {
                                if let Some(si) = tap {
                                    sum += src[si] * layer.weights[w_oi + t];
                                }
                            }
                        }
                        next[o * plane + cell_i] = sum.tanh();
                    }
                }
            }
            cur = next;
            in_ch = layer.out_ch;
        }

        // Head-cell readout ⊕ global average pool.
        let mut feat = Vec::with_capacity(2 * in_ch);
        for c in 0..in_ch {
            feat.push(cur[c * plane + head_i]);
        }
        for c in 0..in_ch {
            let s: f32 = cur[c * plane..c * plane + plane].iter().sum();
            feat.push(s / plane as f32);
        }
        self.head.forward(&feat)
    }

    pub fn to_text(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        out.push_str(MAGIC);
        let _ = writeln!(out, "\nchannels {}", self.in_channels);
        for l in &self.conv_layers {
            let _ = writeln!(out, "conv {} {}", l.in_ch, l.out_ch);
        }
        let dims: Vec<String> = self.head.dims().iter().map(|d| d.to_string()).collect();
        let _ = writeln!(out, "head {}\nparams", dims.join(" "));

        let params = self.flat_params();
        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                out.push(if i % 16 == 0 { '\n' } else { ' ' });
            }
            let _ = write!(out, "{p:?}");
        }
        out.push('\n');
        out
    }

    /// All parameters in storage order: conv layers (weights then biases), then
    /// the head's params.
    fn flat_params(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for l in &self.conv_layers {
            out.extend_from_slice(&l.weights);
            out.extend_from_slice(&l.biases);
        }
        out.extend_from_slice(self.head.params());
        out
    }

    pub fn from_text(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        let magic = lines.next().ok_or("empty file")?.trim();
        if magic != MAGIC {
            return Err(format!("bad magic line: {magic:?}"));
        }

        let mut in_channels: Option<usize> = None;
        let mut conv_shapes: Vec<(usize, usize)> = Vec::new();
        let mut head_dims: Vec<usize> = Vec::new();
        let mut param_lines: Vec<&str> = Vec::new();

        for line in lines {
            let mut tok = line.split_whitespace();
            match tok.next() {
                Some("channels") => {
                    in_channels = Some(parse_one(tok.next(), "channels")?);
                }
                Some("conv") => {
                    let i = parse_one(tok.next(), "conv in_ch")?;
                    let o = parse_one(tok.next(), "conv out_ch")?;
                    conv_shapes.push((i, o));
                }
                Some("head") => {
                    head_dims = tok
                        .map(|t| t.parse().map_err(|e| format!("bad head dim {t:?}: {e}")))
                        .collect::<Result<_, _>>()?;
                }
                Some("params") => break,
                _ => {}
            }
        }
        // Remaining lines (after the `params` marker) are the parameters.
        for line in text.lines().skip_while(|l| l.trim() != "params").skip(1) {
            param_lines.push(line);
        }

        let in_channels = in_channels.ok_or("missing `channels` line")?;
        if head_dims.is_empty() {
            return Err("missing `head` line".into());
        }
        let params: Vec<f32> = param_lines
            .iter()
            .flat_map(|l| l.split_whitespace())
            .map(|t| t.parse().map_err(|e| format!("bad param {t:?}: {e}")))
            .collect::<Result<_, _>>()?;

        Self::from_params(in_channels, &conv_shapes, &head_dims, params)
    }
}

fn parse_one(tok: Option<&str>, what: &str) -> Result<usize, String> {
    tok.ok_or_else(|| format!("missing {what}"))?
        .parse()
        .map_err(|e| format!("bad {what}: {e}"))
}

/// Build the `CONV_CHANNELS × h × w` input planes (channel-major, row-major):
/// body (excl. head), head, food, topology. Mirrors the Python `grid`
/// observation plus the topology plane.
fn build_planes(state: &GameState) -> (Vec<f32>, usize, usize) {
    let board = state.board();
    let (w, h) = (board.width as usize, board.height as usize);
    let plane = w * h;
    let mut planes = vec![0.0f32; CONV_CHANNELS * plane];
    let head = state.head();
    let idx = |c: Offset| (c.row as usize) * w + c.col as usize;

    for cell in state.snake() {
        let ch = if cell == head { 1 } else { 0 };
        planes[ch * plane + idx(cell)] = 1.0;
    }
    planes[2 * plane + idx(state.food())] = 1.0;
    let topo = match board.boundary {
        BoundaryMode::Walls => 1.0,
        BoundaryMode::Periodic => 0.0,
    };
    for v in &mut planes[3 * plane..4 * plane] {
        *v = topo;
    }
    (planes, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Config, GameState};

    /// A 1-channel, 1-layer net whose kernel reads only the center tap with
    /// weight 1 (bias 0); head is identity-ish so we can check the math.
    fn center_identity_net() -> HexConv {
        // conv 1→1: weights [center=1, six neighbors=0], bias 0.
        let mut params = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        // head 2→1: weights [1, 0] (take only the readout), bias 0.
        params.extend_from_slice(&[1.0, 0.0, 0.0]);
        HexConv::from_params(1, &[(1, 1)], &[2, 1], params).unwrap()
    }

    #[test]
    fn center_tap_passes_value_through_tanh() {
        let net = center_identity_net();
        // 1×1×3 board, head at cell 0 carrying value 0.5.
        let planes = vec![0.5, 0.0, 0.0];
        let out = net.forward_planes(&planes, 3, 1, BoundaryMode::Walls, 0);
        // conv: tanh(0.5); head: linear 1×tanh(0.5).
        assert!((out[0] - 0.5f32.tanh()).abs() < 1e-6);
    }

    #[test]
    fn walls_zero_pad_vs_torus_wrap() {
        // Kernel that sums all six neighbors (center weight 0, neighbor 1).
        let mut params = vec![0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0];
        params.extend_from_slice(&[1.0, 0.0, 0.0]); // head: take readout
        let net = HexConv::from_params(1, &[(1, 1)], &[2, 1], params).unwrap();

        // Every cell lit, read at the corner. The conv sums the corner's
        // neighbors: on walls some fall off-board (zero), on the torus all six
        // wrap back and are lit. So the corner's neighbor-sum (and thus the
        // tanh output) differs between the two topologies.
        let w = 4;
        let h = 4;
        let planes = vec![1.0f32; w * h];
        let walls = net.forward_planes(&planes, w, h, BoundaryMode::Walls, 0); // corner cell
        let torus = net.forward_planes(&planes, w, h, BoundaryMode::Periodic, 0);
        assert_ne!(
            walls, torus,
            "boundary handling must change the convolution"
        );
        assert!(
            (torus[0] - 6.0f32.tanh()).abs() < 1e-6,
            "torus corner has 6 lit neighbors"
        );
    }

    #[test]
    fn text_roundtrip_is_exact() {
        let n_conv = ConvLayer::param_count(4, 6) + ConvLayer::param_count(6, 6);
        let head_dims = [12usize, 8, 6];
        let n = n_conv + Mlp::param_count(&head_dims);
        let params: Vec<f32> = (0..n).map(|i| (i as f32 - 7.0) * 0.013).collect();
        let net = HexConv::from_params(4, &[(4, 6), (6, 6)], &head_dims, params).unwrap();
        let restored = HexConv::from_text(&net.to_text()).unwrap();
        assert_eq!(net, restored);
    }

    #[test]
    fn rejects_bad_shapes() {
        // Head input must be 2×last_out.
        assert!(HexConv::from_params(4, &[(4, 6)], &[6, 6], vec![0.0; 999]).is_err());
        // Broken channel chain.
        assert!(HexConv::from_params(4, &[(4, 6), (8, 8)], &[16, 6], vec![0.0; 999]).is_err());
        assert!(HexConv::from_text("nope").is_err());
    }

    #[test]
    fn forward_on_state_is_size_agnostic_and_deterministic() {
        let n_conv = ConvLayer::param_count(CONV_CHANNELS, 6) + ConvLayer::param_count(6, 6);
        let head_dims = [12usize, 8, 6];
        let n = n_conv + Mlp::param_count(&head_dims);
        let params: Vec<f32> = (0..n).map(|i| ((i % 7) as f32 - 3.0) * 0.05).collect();
        let net = HexConv::from_params(
            CONV_CHANNELS,
            &[(CONV_CHANNELS, 6), (6, 6)],
            &head_dims,
            params,
        )
        .unwrap();

        for (w, h) in [(16, 12), (24, 18)] {
            let state = GameState::new(Config {
                width: w,
                height: h,
                ..Config::default()
            });
            let a = net.forward(&state);
            let b = net.forward(&state);
            assert_eq!(a.len(), 6);
            assert_eq!(a, b, "deterministic on {w}×{h}");
        }
    }
}
