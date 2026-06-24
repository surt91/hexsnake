"""Smoke trainer for the hex-conv ConvNet via behavior cloning.

The agent only runs tiny smoke runs to verify the train -> export -> play
pipeline; the user runs the real training on stronger hardware (see
``docs/training/cnn/guide.md``). This script clones a cheap BFS-to-food expert
into a `HexConvNet` and exports the `.cnn` weights.

Why BFS over the Rust ``neighbor_table``: it gives true hop distance including
torus wrap for free, so we don't re-derive hex/torus math in Python. The expert
is deliberately simple (shortest safe step toward food) — good enough to prove
the supervised loop produces a loadable, board-aware net.

Run:  python train_cnn.py [--games N] [--epochs E] [--out PATH]
"""

from __future__ import annotations

import argparse
from collections import deque

import numpy as np
import torch as th
import torch.nn as nn

from hexsnake_rl import HexSnakeGym, _native
from hexsnake_rl.export import export_cnn
from hexsnake_rl.hexconv import HexConvNet

W, H, IN_CH = 16, 12, 4
BOUNDARIES = ["walls", "torus"]


def bfs_step(neigh, head, food, body, n):
    """Tap index (0..5) of the first step on a shortest body-free path from
    `head` to `food`, or None if unreachable."""
    prev = {head: (head, -1)}
    q = deque([head])
    while q:
        c = q.popleft()
        if c == food:
            break
        for t in range(6):
            nb = neigh[c * 6 + t]
            if nb < 0 or nb in prev or (nb in body and nb != food):
                continue
            prev[nb] = (c, t)
            q.append(nb)
    if food not in prev:
        return None
    cur = food
    while prev[cur][0] != head:
        cur = prev[cur][0]
        if cur == head:
            break
    return prev[cur][1]


def collect(games: int):
    """Drive games with the expert, recording (planes, head_idx, abs_label)."""
    rng = np.random.default_rng(0)
    plane = W * H
    samples = []
    for g in range(games):
        boundary = BOUNDARIES[g % 2]
        neigh = _native.neighbor_table(W, H, boundary)
        env = HexSnakeGym(W, H, boundary, max_ticks=400, observation="grid")
        obs, _ = env.reset(seed=int(rng.integers(0, 1 << 30)))
        done = False
        while not done:
            grid = np.asarray(obs, dtype=np.float32).reshape(3, plane)
            head = int(grid[1].argmax())
            food = int(grid[2].argmax())
            body = {i for i in range(plane) if grid[0, i] > 0.5}
            # Heading: the body neighbor of the head is the neck (opposite the
            # heading); recover the heading tap to convert abs -> relative.
            neck_tap = next(
                (t for t in range(6) if neigh[head * 6 + t] in body and neigh[head * 6 + t] != head),
                None,
            )
            if neck_tap is None:
                break
            heading = (neck_tap + 3) % 6

            tap = bfs_step(neigh, head, food, body, plane)
            if tap is None:
                # No safe path: pick any safe neighbor to keep moving.
                tap = next(
                    (t for t in range(6) if neigh[head * 6 + t] >= 0 and neigh[head * 6 + t] not in body),
                    0,
                )

            planes = np.zeros((IN_CH, plane), dtype=np.float32)
            planes[:3] = grid
            planes[3] = 1.0 if boundary == "walls" else 0.0
            samples.append((planes, head, tap))

            action = (tap - heading) % 6  # absolute tap -> relative action
            obs, _r, term, trunc, _s = env.step(action)
            done = term or trunc
    return samples


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--games", type=int, default=20)
    ap.add_argument("--epochs", type=int, default=5)
    ap.add_argument("--out", default="training-out/cnn/best.cnn")
    args = ap.parse_args()

    samples = collect(args.games)
    print(f"collected {len(samples)} expert states")
    planes = th.from_numpy(np.stack([s[0] for s in samples]))
    heads = th.tensor([s[1] for s in samples])
    labels = th.tensor([s[2] for s in samples])

    # Train on a fixed board geometry; exported weights stay size-agnostic.
    model = HexConvNet(W, H, "walls", IN_CH, [(4, 8), (8, 8)], [16, 12, 6])
    opt = th.optim.Adam(model.parameters(), lr=1e-3)
    loss_fn = nn.CrossEntropyLoss()
    for e in range(args.epochs):
        opt.zero_grad()
        logits = model(planes, heads)
        loss = loss_fn(logits, labels)
        loss.backward()
        opt.step()
        print(f"epoch {e}: loss {loss.item():.4f}")

    export_cnn(args.out, model)
    print(f"exported {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
