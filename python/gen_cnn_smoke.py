#!/usr/bin/env python3
"""Generate small deterministic smoke-run `.cnn` weight files.

These are untrained placeholders (like the GA/AZ smoke artifacts) so the Rust
crate compiles and the strategies play legal moves. Real weights come from a
training run — see ``docs/training/cnn/guide.md``. The parameter ordering must
match ``HexConv::from_params`` on the Rust side: per conv layer the
``out×in×7`` kernel (row-major) followed by ``out`` biases, then the dense
head's ``Mlp`` params.
"""

from __future__ import annotations

import random
from pathlib import Path

TAPS = 7
ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "crates" / "snake-core" / "assets"


def conv_param_count(in_ch: int, out_ch: int) -> int:
    return out_ch * in_ch * TAPS + out_ch


def mlp_param_count(dims: list[int]) -> int:
    return sum(a * b + b for a, b in zip(dims, dims[1:]))


def gen(channels: int, convs: list[tuple[int, int]], head: list[int], seed: int) -> str:
    rng = random.Random(seed)
    n = sum(conv_param_count(i, o) for i, o in convs) + mlp_param_count(head)
    # Small weights keep the untrained net's outputs in a sane range.
    params = [round(rng.gauss(0.0, 0.3), 6) for _ in range(n)]

    lines = ["hexsnake-cnn v1", f"channels {channels}"]
    lines += [f"conv {i} {o}" for i, o in convs]
    lines.append("head " + " ".join(str(d) for d in head))
    lines.append("params")
    for i in range(0, n, 16):
        lines.append(" ".join(repr(p) for p in params[i : i + 16]))
    return "\n".join(lines) + "\n"


def main() -> None:
    # Standalone ConvNet: six absolute direction scores.
    conv_net = gen(4, [(4, 8), (8, 8)], [16, 12, 6], seed=1)
    (ASSETS / "cnn").mkdir(parents=True, exist_ok=True)
    (ASSETS / "cnn" / "best.cnn").write_text(conv_net)

    # AlphaZero-conv: six policy logits + one value head.
    az_conv = gen(4, [(4, 8), (8, 8)], [16, 12, 7], seed=2)
    (ASSETS / "alphazero-cnn").mkdir(parents=True, exist_ok=True)
    (ASSETS / "alphazero-cnn" / "best.cnn").write_text(az_conv)

    print("wrote cnn/best.cnn and alphazero-cnn/best.cnn")


if __name__ == "__main__":
    main()
