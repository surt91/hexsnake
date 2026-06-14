"""Gradient-trained AlphaZero-light.

Self-play runs entirely in Rust (`az_selfplay`) using the *same* MCTS the game
plays at inference — there is no second search to keep in sync. The GIL is
released during each game, so a thread pool fans self-play out across all
cores. Python only does the gradient step: policy loss = cross-entropy against
the MCTS visit distribution, value loss = MSE against the (tanh) return.

The trained net is exported to the `.mlp` format and inference stays pure
Rust/WASM (`AlphaZeroLite`). Requires the `train` extra (torch).

Example:
    uv run --extra train python train_alphazero.py --iterations 30 \
        --games-per-iter 96 --sims 24 --boundary mixed --out az.mlp
"""

import argparse
import os
import random
import tempfile
from collections import deque
from concurrent.futures import ThreadPoolExecutor

import numpy as np
import torch
import torch.nn.functional as F

from hexsnake_rl import az_selfplay, mlp_forward
from hexsnake_rl.az_net import ACTIONS, AZNet


def export_self_check(net: AZNet) -> None:
    """Assert the exported .mlp reproduces the net's raw output in Rust."""
    fd, path = tempfile.mkstemp(suffix=".mlp")
    os.close(fd)
    net.export(path)
    text = open(path, encoding="utf-8").read()
    os.remove(path)
    x = np.random.randn(20).astype(np.float32)
    with torch.no_grad():
        want = net.raw(torch.from_numpy(x)).numpy()
    got = np.asarray(mlp_forward(text, x.tolist()), dtype=np.float32)
    err = float(np.max(np.abs(want - got)))
    assert err < 1e-4, f"AZ net export/Rust mismatch: {err}"
    print(f"export self-check OK (err {err:.2e})")


def collect_self_play(net, args, base_seed):
    """Run `--games-per-iter` self-play games in parallel; return sample rows."""
    params = net.to_params()
    boundaries = (
        ["walls", "torus"] if args.boundary == "mixed" else [args.boundary]
    )

    def one(i):
        boundary = boundaries[i % len(boundaries)]
        return az_selfplay(
            params, boundary, 16, 12, args.sims, args.temperature,
            base_seed + i, args.max_ticks,
        )

    rows = []
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        for res in pool.map(one, range(args.games_per_iter)):
            rows.extend(res)
    return rows


def train_step(net, optim, batch):
    feats, policy_t, value_t = batch
    logits, value = net(feats)
    # Cross-entropy over the five non-reverse actions (reverse is masked).
    logp = F.log_softmax(logits[:, ACTIONS], dim=-1)
    policy_loss = -(policy_t[:, ACTIONS] * logp).sum(dim=-1).mean()
    value_loss = F.mse_loss(value, value_t)
    loss = policy_loss + value_loss
    optim.zero_grad()
    loss.backward()
    optim.step()
    return policy_loss.item(), value_loss.item()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--iterations", type=int, default=30)
    ap.add_argument("--games-per-iter", type=int, default=96)
    ap.add_argument("--sims", type=int, default=24)
    ap.add_argument("--temperature", type=float, default=1.0)
    ap.add_argument("--boundary", default="mixed", choices=["walls", "torus", "mixed"])
    ap.add_argument("--max-ticks", type=int, default=1500)
    ap.add_argument("--epochs", type=int, default=4)
    ap.add_argument("--batch", type=int, default=256)
    ap.add_argument("--lr", type=float, default=1e-3)
    ap.add_argument("--buffer", type=int, default=60000)
    ap.add_argument("--workers", type=int, default=os.cpu_count() or 4)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--out", default="az.mlp")
    args = ap.parse_args()

    torch.manual_seed(args.seed)
    random.seed(args.seed)
    torch.set_num_threads(1)  # parallelism is in Rust self-play, not torch

    net = AZNet()
    export_self_check(net)
    optim = torch.optim.Adam(net.parameters(), lr=args.lr)
    buffer = deque(maxlen=args.buffer)
    seed = args.seed * 1_000_000

    for it in range(args.iterations):
        rows = collect_self_play(net, args, seed)
        seed += args.games_per_iter
        buffer.extend(rows)

        feats = torch.tensor(np.array([r[0] for r in buffer], dtype=np.float32))
        policy_t = torch.tensor(np.array([r[1] for r in buffer], dtype=np.float32))
        value_t = torch.tensor(np.array([r[2] for r in buffer], dtype=np.float32))

        n = feats.shape[0]
        last = (0.0, 0.0)
        for _ in range(args.epochs):
            perm = torch.randperm(n)
            for s in range(0, n, args.batch):
                idx = perm[s : s + args.batch]
                last = train_step(net, optim, (feats[idx], policy_t[idx], value_t[idx]))

        avg_game_len = len(rows) / args.games_per_iter
        print(
            f"iter {it:3d}  buffer {n:6d}  ~game_len {avg_game_len:6.1f}  "
            f"policy_loss {last[0]:.3f}  value_loss {last[1]:.3f}"
        )

    net.export(args.out)
    print(f"exported policy/value net -> {args.out}")


if __name__ == "__main__":
    main()
