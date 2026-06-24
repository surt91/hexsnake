"""Verify the `.cnn` weight-format roundtrip against the real Rust inference.

Builds a random `HexConvNet`, exports it to the `.cnn` format, then for many
random board states compares the torch forward pass with `snake-core`'s
hex-conv forward (via `_native.cnn_forward`). Guards the Python->Rust layout
(the conv kernel order, the readout⊕pool concatenation, the head) the same way
`verify_roundtrip.py` guards the `.mlp` format.

Run:  python verify_cnn_roundtrip.py
"""

import os
import tempfile

import numpy as np
import torch as th

from hexsnake_rl import _native
from hexsnake_rl.export import export_cnn
from hexsnake_rl.hexconv import HexConvNet


def main() -> int:
    th.manual_seed(0)
    rng = np.random.default_rng(0)
    w, h, boundary = 16, 12, "walls"
    n = w * h
    in_ch = 4

    model = HexConvNet(w, h, boundary, in_ch, [(4, 8), (8, 8)], [16, 12, 7])
    model.eval()

    with tempfile.TemporaryDirectory() as d:
        path = os.path.join(d, "roundtrip.cnn")
        export_cnn(path, model)
        text = open(path, encoding="utf-8").read()

    max_err = 0.0
    for _ in range(40):
        planes = rng.standard_normal((in_ch, n)).astype(np.float32)
        head_idx = int(rng.integers(0, n))

        with th.no_grad():
            want = model(
                th.from_numpy(planes)[None], th.tensor([head_idx])
            )[0].numpy()
        got = np.asarray(
            _native.cnn_forward(text, planes.reshape(-1).tolist(), w, h, head_idx, boundary),
            dtype=np.float32,
        )
        max_err = max(max_err, float(np.max(np.abs(want - got))))

    print(f"max abs error torch vs rust: {max_err:.2e}")
    assert max_err < 1e-4, "weight layout mismatch between Python export and Rust!"
    print("OK: Python hex-conv export and Rust inference agree.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
