"""Verify the weight-format roundtrip WITHOUT torch.

Builds a random MLP with numpy, exports it to the `.mlp` format, runs the
identical forward pass in numpy and in the actual Rust inference (via
`hexsnake_env.mlp_forward`), and asserts they agree. This guards the
Python->Rust layout (the classic transposition/ordering trap) independently
of stable-baselines3.

Run:  python verify_roundtrip.py
"""

import os
import tempfile

import numpy as np

from hexsnake_rl import mlp_forward
from hexsnake_rl.export import export_mlp


def numpy_forward(layers, x):
    a = np.asarray(x, dtype=np.float32)
    for i, (W, b) in enumerate(layers):
        a = W @ a + b
        if i < len(layers) - 1:
            a = np.tanh(a)  # hidden layers
    return a


def main() -> int:
    rng = np.random.default_rng(0)
    dims = [20, 32, 24, 6]
    layers = [
        (
            rng.standard_normal((dims[i + 1], dims[i])).astype(np.float32),
            rng.standard_normal(dims[i + 1]).astype(np.float32),
        )
        for i in range(len(dims) - 1)
    ]

    with tempfile.TemporaryDirectory() as d:
        path = os.path.join(d, "roundtrip.mlp")
        export_mlp(path, layers)
        text = open(path, encoding="utf-8").read()

    max_err = 0.0
    for _ in range(50):
        x = rng.standard_normal(dims[0]).astype(np.float32)
        want = numpy_forward(layers, x)
        got = np.asarray(mlp_forward(text, x.tolist()), dtype=np.float32)
        max_err = max(max_err, float(np.max(np.abs(want - got))))

    print(f"max abs error numpy vs rust: {max_err:.2e}")
    assert max_err < 1e-4, "weight layout mismatch between Python export and Rust!"
    print("OK: Python export and Rust inference agree.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
