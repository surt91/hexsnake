//! Tiny MLP with a pure-Rust forward pass and a simple, documented text
//! weight format (also produced by the later Python exporters).
//!
//! # Weight format (`.mlp`, UTF-8 text)
//!
//! ```text
//! hexsnake-mlp v1          <- magic + version
//! 19 16 12 6               <- layer dims, input first
//! 0.123 -0.5 …             <- all parameters, whitespace-separated:
//!                             per layer: weights row-major (out × in),
//!                             then biases; layers in order
//! ```
//!
//! Hidden layers use tanh, the output layer is linear.

#[derive(Debug, Clone, PartialEq)]
pub struct Mlp {
    dims: Vec<usize>,
    /// Flat parameters: for each layer, weights (out×in, row-major), then
    /// biases.
    params: Vec<f32>,
}

const MAGIC: &str = "hexsnake-mlp v1";

impl Mlp {
    /// Number of parameters for the given layer dimensions.
    pub fn param_count(dims: &[usize]) -> usize {
        dims.windows(2).map(|w| w[0] * w[1] + w[1]).sum()
    }

    /// Build from a flat parameter vector (e.g. a GA genome).
    pub fn from_params(dims: &[usize], params: Vec<f32>) -> Result<Self, String> {
        if dims.len() < 2 {
            return Err("need at least input and output layer".into());
        }
        let expected = Self::param_count(dims);
        if params.len() != expected {
            return Err(format!(
                "expected {expected} params for dims {dims:?}, got {}",
                params.len()
            ));
        }
        Ok(Self {
            dims: dims.to_vec(),
            params,
        })
    }

    pub fn dims(&self) -> Vec<usize> {
        self.dims.clone()
    }

    pub fn params(&self) -> &[f32] {
        &self.params
    }

    pub fn forward(&self, input: &[f32]) -> Vec<f32> {
        assert_eq!(input.len(), self.dims[0], "input dimension mismatch");
        let mut activation = input.to_vec();
        let mut offset = 0;
        let last_layer = self.dims.len() - 2;
        for (layer, w) in self.dims.windows(2).enumerate() {
            let (n_in, n_out) = (w[0], w[1]);
            let weights = &self.params[offset..offset + n_in * n_out];
            let biases = &self.params[offset + n_in * n_out..offset + n_in * n_out + n_out];
            offset += n_in * n_out + n_out;

            let mut next = Vec::with_capacity(n_out);
            for o in 0..n_out {
                let row = &weights[o * n_in..(o + 1) * n_in];
                let mut sum = biases[o];
                for (x, wgt) in activation.iter().zip(row) {
                    sum += x * wgt;
                }
                next.push(if layer < last_layer { sum.tanh() } else { sum });
            }
            activation = next;
        }
        activation
    }

    pub fn to_text(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        out.push_str(MAGIC);
        out.push('\n');
        let dims: Vec<String> = self.dims.iter().map(|d| d.to_string()).collect();
        out.push_str(&dims.join(" "));
        out.push('\n');
        for (i, p) in self.params.iter().enumerate() {
            if i > 0 {
                out.push(if i % 16 == 0 { '\n' } else { ' ' });
            }
            // `{:?}` prints the shortest representation that round-trips.
            let _ = write!(out, "{p:?}");
        }
        out.push('\n');
        out
    }

    pub fn from_text(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        let magic = lines.next().ok_or("empty file")?.trim();
        if magic != MAGIC {
            return Err(format!("bad magic line: {magic:?}"));
        }
        let dims: Vec<usize> = lines
            .next()
            .ok_or("missing dims line")?
            .split_whitespace()
            .map(|t| t.parse().map_err(|e| format!("bad dim {t:?}: {e}")))
            .collect::<Result<_, _>>()?;
        let params: Vec<f32> = lines
            .flat_map(|l| l.split_whitespace())
            .map(|t| t.parse().map_err(|e| format!("bad param {t:?}: {e}")))
            .collect::<Result<_, _>>()?;
        Self::from_params(&dims, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn param_count_matches_layout() {
        // 2-3-1: (2*3+3) + (3*1+1) = 13
        assert_eq!(Mlp::param_count(&[2, 3, 1]), 13);
    }

    #[test]
    fn forward_known_values() {
        // Single layer 2 -> 1, weights [1, -1], bias 0.5: linear output.
        let mlp = Mlp::from_params(&[2, 1], vec![1.0, -1.0, 0.5]).unwrap();
        let out = mlp.forward(&[2.0, 1.0]);
        assert_eq!(out, vec![1.5]);
    }

    #[test]
    fn hidden_layers_use_tanh() {
        // 1-1-1, all weights 1, biases 0: out = tanh(x).
        let mlp = Mlp::from_params(&[1, 1, 1], vec![1.0, 0.0, 1.0, 0.0]).unwrap();
        let out = mlp.forward(&[0.5]);
        assert!((out[0] - 0.5f32.tanh()).abs() < 1e-6);
    }

    #[test]
    fn text_roundtrip_is_exact() {
        let params: Vec<f32> = (0..13).map(|i| (i as f32 - 6.0) * 0.317).collect();
        let mlp = Mlp::from_params(&[2, 3, 1], params).unwrap();
        let restored = Mlp::from_text(&mlp.to_text()).unwrap();
        assert_eq!(mlp, restored, "f32 debug formatting must round-trip");
    }

    #[test]
    fn rejects_garbage() {
        assert!(Mlp::from_text("nope").is_err());
        assert!(Mlp::from_text("hexsnake-mlp v1\n2 1\n1.0").is_err()); // too few params
        assert!(Mlp::from_params(&[3], vec![]).is_err());
    }
}
