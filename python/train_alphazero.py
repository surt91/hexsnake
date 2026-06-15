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
    """Run `--games-per-iter` self-play games in parallel.

    Returns `(rows, mean_score, mean_ticks)`: the flattened training samples
    plus per-game averages. These averages are *only logged* to watch training
    progress — checkpoint selection uses the greedy benchmark (`eval_net`)
    instead, because stochastic, tick-capped self-play scores stay high even
    when the greedy policy has collapsed into circling without eating.
    """
    params = net.to_params()
    boundaries = (
        ["walls", "torus"] if args.boundary == "mixed" else [args.boundary]
    )

    dims = net.get_dims()

    def one(i):
        boundary = boundaries[i % len(boundaries)]
        return az_selfplay(
            params, boundary, 16, 12, args.sims, args.temperature,
            base_seed + i, args.max_ticks,
            args.eat_bonus, args.sp_eat,
            dims,
        )

    rows = []
    scores, tick_counts = [], []
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        for game_rows, score, ticks in pool.map(one, range(args.games_per_iter)):
            rows.extend(game_rows)
            scores.append(score)
            tick_counts.append(ticks)
    mean_score = sum(scores) / len(scores)
    mean_ticks = sum(tick_counts) / len(tick_counts)
    return rows, mean_score, mean_ticks


# Fixed eval seeds, disjoint from the training-seed range, so the greedy
# benchmark is deterministic and comparable across iterations and runs.
EVAL_SEED_BASE = 7_000_000


def eval_net(net, args):
    """Greedy benchmark proxy used for checkpoint selection.

    Plays `--eval-games` *greedy* (temperature 0) games per topology on fixed
    seeds at the deployment tick budget, mirroring what `bench_mlp` measures.
    Returns `(walls, torus, mean, ticks)` average scores; `mean` drives
    selection. Unlike self-play this exposes circling collapse (high ticks,
    low score), so the saved checkpoint matches the deployed objective.
    """
    params = net.to_params()
    dims = net.get_dims()

    def one(job):
        boundary, seed = job
        _, score, ticks = az_selfplay(
            params, boundary, 16, 12, args.sims, 0.0,  # temperature 0 = greedy
            seed, args.eval_max_ticks, args.eat_bonus, args.sp_eat, dims,
        )
        return boundary, score, ticks

    jobs = [
        (boundary, EVAL_SEED_BASE + i)
        for boundary in ("walls", "torus")
        for i in range(args.eval_games)
    ]
    scores = {"walls": [], "torus": []}
    tick_counts = []
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        for boundary, score, ticks in pool.map(one, jobs):
            scores[boundary].append(score)
            tick_counts.append(ticks)
    walls = sum(scores["walls"]) / len(scores["walls"])
    torus = sum(scores["torus"]) / len(scores["torus"])
    ticks = sum(tick_counts) / len(tick_counts)
    return walls, torus, (walls + torus) / 2, ticks


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


def load_net_from_mlp(path: str) -> "AZNet":
    """Warm-start an AZNet from an exported .mlp file (any hidden size)."""
    with open(path, encoding="utf-8") as f:
        lines = [ln.strip() for ln in f if ln.strip()]
    dims = list(map(int, lines[1].split()))
    hidden = tuple(dims[1:-1])  # strip input (20) and output (7) dims
    params: list[float] = []
    for line in lines[2:]:
        params.extend(float(v) for v in line.split())

    net = AZNet(hidden=hidden)
    offset = 0
    for (W_param, b_param), in_dim, out_dim in zip(net._layers(), dims, dims[1:]):
        n_w = out_dim * in_dim
        W_data = torch.tensor(
            params[offset : offset + n_w], dtype=torch.float32
        ).reshape(out_dim, in_dim)
        offset += n_w
        b_data = torch.tensor(
            params[offset : offset + out_dim], dtype=torch.float32
        )
        offset += out_dim
        with torch.no_grad():
            W_param.copy_(W_data)
            b_param.copy_(b_data)
    return net


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
    ap.add_argument("--eat-bonus", type=float, default=0.3,
                    help="MCTS edge reward for eating food (default 0.3)")
    ap.add_argument("--sp-eat", type=float, default=1.0,
                    help="self-play return bonus for eating food (default 1.0)")
    ap.add_argument("--eval-every", type=int, default=5,
                    help="run the greedy benchmark for checkpoint selection every N iters")
    ap.add_argument("--eval-games", type=int, default=12,
                    help="greedy eval games per topology (Walls + Periodic)")
    ap.add_argument("--eval-max-ticks", type=int, default=3000,
                    help="tick budget per greedy eval game")
    ap.add_argument("--out", default="az.mlp")
    ap.add_argument(
        "--best-out",
        default=None,
        help="path for the best-checkpoint (by avg_game_len); defaults to <out>.best.mlp",
    )
    ap.add_argument(
        "--load",
        default=None,
        metavar="MLP",
        help="warm-start from an existing .mlp file instead of random weights",
    )
    ap.add_argument(
        "--hidden",
        type=int,
        nargs="+",
        default=[32, 24],
        metavar="N",
        help="hidden layer sizes (default: 32 24)",
    )
    args = ap.parse_args()

    best_out = args.best_out or (args.out.removesuffix(".mlp") + ".best.mlp")

    torch.manual_seed(args.seed)
    random.seed(args.seed)
    torch.set_num_threads(1)  # parallelism is in Rust self-play, not torch

    if args.load:
        net = load_net_from_mlp(args.load)
        print(f"loaded initial weights from {args.load}")
    else:
        net = AZNet(hidden=tuple(args.hidden))
    export_self_check(net)
    optim = torch.optim.Adam(net.parameters(), lr=args.lr)
    buffer = deque(maxlen=args.buffer)
    seed = args.seed * 1_000_000

    best_score = -1.0
    best_iter = -1

    for it in range(args.iterations):
        rows, mean_score, mean_ticks = collect_self_play(net, args, seed)
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

        # Checkpoint on the *greedy benchmark* (what we deploy), not on the
        # stochastic, tick-capped self-play score: the latter stays high while
        # the greedy policy collapses into circling, which is how earlier runs
        # saved worse-than-final checkpoints and turned results into a seed
        # lottery. Eval runs every --eval-every iters and on the final iter.
        eval_str = ""
        is_best = False
        if it % args.eval_every == 0 or it == args.iterations - 1:
            ev_walls, ev_torus, ev_mean, ev_ticks = eval_net(net, args)
            eval_str = (
                f"  eval[W {ev_walls:5.1f} P {ev_torus:5.1f} "
                f"avg {ev_mean:5.1f} t {ev_ticks:5.0f}]"
            )
            is_best = ev_mean > best_score
            if is_best:
                best_score = ev_mean
                best_iter = it
                net.export(best_out)

        print(
            f"iter {it:3d}  buffer {n:6d}  sp_score {mean_score:5.2f}  "
            f"sp_ticks {mean_ticks:6.1f}  "
            f"ploss {last[0]:.3f}  vloss {last[1]:.3f}"
            + eval_str
            + ("  [best]" if is_best else "")
        )

    net.export(args.out)
    print(f"exported policy/value net -> {args.out}")
    print(f"best checkpoint (iter {best_iter}, eval_avg {best_score:.2f}) -> {best_out}")


if __name__ == "__main__":
    main()
