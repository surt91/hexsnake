"""Gradient-trained AlphaZero-Conv (whole-board vision).

The conv analogue of `train_alphazero.py`: self-play runs entirely in Rust
(`az_conv_selfplay`) using the *same* MCTS the game plays at inference, with the
`HexConvNet` (policy+value) as the evaluator — there is no second search to keep
in sync. The GIL is released during each game, so a thread pool fans self-play
out across all cores. Python only does the gradient step: policy loss =
cross-entropy against the MCTS visit distribution (in the **absolute** direction
frame the conv head predicts, masking the per-state reverse move), value loss =
MSE against the (tanh) return.

The trained net is exported to the `.cnn` format and inference stays pure
Rust/WASM (`AlphaZeroConv`). Requires the `train` extra (torch).

Example:
    uv run --extra train python train_alphazero_conv.py --iterations 60 \
        --games-per-iter 96 --sims 24 --boundary mixed --out az-conv.cnn
"""

from __future__ import annotations

import argparse
import os
import random
import time
from collections import deque
from concurrent.futures import ThreadPoolExecutor

import numpy as np
import torch as th
import torch.nn.functional as F

from hexsnake_rl import _native, az_conv_selfplay
from hexsnake_rl.export import cnn_text, export_cnn
from hexsnake_rl.hexconv import HexConvNet

W, H, IN_CH = 16, 12, 4
PLANE = W * H
BOUNDARIES = ["walls", "torus"]  # tid 0 = walls, tid 1 = torus
OUTPUTS = 7  # 6 absolute policy logits + 1 value


def export_self_check(model, gather) -> None:
    """Assert the exported .cnn reproduces the net's raw output in Rust."""
    model.eval()
    text = cnn_text(model)
    rng = np.random.default_rng(0)
    max_err = 0.0
    for _ in range(16):
        planes = rng.standard_normal((IN_CH, PLANE)).astype(np.float32)
        head_idx = int(rng.integers(0, PLANE))
        with th.no_grad():
            want = model(th.from_numpy(planes)[None], th.tensor([head_idx]), gather)[0].numpy()
        got = np.asarray(
            _native.cnn_forward(text, planes.reshape(-1).tolist(), W, H, head_idx, "walls"),
            dtype=np.float32,
        )
        max_err = max(max_err, float(np.max(np.abs(want - got))))
    assert max_err < 1e-4, f"conv export/Rust mismatch: {max_err}"
    print(f"export self-check OK (err {max_err:.2e})")


def collect_self_play(text, args, base_seed):
    """Run `--games-per-iter` self-play games in parallel.

    Returns `(rows, mean_score, mean_ticks)` where each row is
    `(grid, policy, value, heading, tid)`. The per-game averages are *only
    logged* — checkpoint selection uses the greedy benchmark (`eval_net`), since
    stochastic, tick-capped self-play scores stay high even when the greedy
    policy has collapsed into circling.
    """
    boundaries = ["walls", "torus"] if args.boundary == "mixed" else [args.boundary]

    def one(i):
        boundary = boundaries[i % len(boundaries)]
        tid = BOUNDARIES.index(boundary)
        game_rows, score, ticks = az_conv_selfplay(
            text, boundary, W, H, args.sims, args.temperature,
            base_seed + i, args.max_ticks, args.eat_bonus, args.sp_eat,
        )
        return [(g, p, v, hd, tid) for (g, p, v, hd) in game_rows], score, ticks

    rows = []
    scores, tick_counts = [], []
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        for game_rows, score, ticks in pool.map(one, range(args.games_per_iter)):
            rows.extend(game_rows)
            scores.append(score)
            tick_counts.append(ticks)
    return rows, sum(scores) / len(scores), sum(tick_counts) / len(tick_counts)


def eval_net(model, args):
    """Greedy benchmark proxy used for checkpoint selection.

    Plays `--eval-games` *greedy* (temperature 0) games per topology on board
    seeds `0..eval_games` — the seeds `bench_mlp`/the benchmark use — at the
    deployment tick budget. Returns `(walls, torus, mean, ticks)`; `mean` drives
    selection. Training self-play uses board seeds >= seed*1e6, so these eval
    boards stay held out. Unlike self-play this exposes circling collapse (high
    ticks, low score), so the saved checkpoint matches the deployed objective.
    """
    text = cnn_text(model)

    def one(job):
        boundary, seed = job
        _, score, ticks = az_conv_selfplay(
            text, boundary, W, H, args.sims, 0.0,  # temperature 0 = greedy
            seed, args.eval_max_ticks, args.eat_bonus, args.sp_eat,
        )
        return boundary, score, ticks

    jobs = [(b, i) for b in BOUNDARIES for i in range(args.eval_games)]
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


def train_step(model, opt, gather, batch):
    planes, heads, policy_t, value_t, heading = batch
    out = model(planes, heads, gather)
    policy_logits, value = out[:, :6], out[:, 6]
    # Mask the per-state reverse move (auto-overridden in game, never searched):
    # the absolute-frame analogue of the MLP trainer's fixed reverse mask. The
    # target is already 0 there, and a finite -1e9 (not -inf) keeps 0*logp = 0.
    reverse = (heading + 3) % 6
    masked = policy_logits.masked_fill(
        F.one_hot(reverse, 6).bool(), -1e9
    )
    logp = F.log_softmax(masked, dim=-1)
    policy_loss = -(policy_t * logp).sum(dim=-1).mean()
    value_loss = F.mse_loss(value, value_t)
    loss = policy_loss + value_loss
    opt.zero_grad()
    loss.backward()
    opt.step()
    return policy_loss.item(), value_loss.item()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--iterations", type=int, default=60)
    ap.add_argument("--games-per-iter", type=int, default=96)
    ap.add_argument("--sims", type=int, default=24)
    ap.add_argument("--temperature", type=float, default=1.0)
    ap.add_argument("--boundary", default="mixed", choices=["walls", "torus", "mixed"])
    ap.add_argument("--max-ticks", type=int, default=1500)
    ap.add_argument("--epochs", type=int, default=4)
    ap.add_argument("--batch", type=int, default=256)
    ap.add_argument("--lr", type=float, default=1e-3)
    ap.add_argument("--buffer", type=int, default=120000)
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
    ap.add_argument("--max-hours", type=float, default=0.0,
                    help="wall-clock budget in hours (0 = run all --iterations); "
                         "stops after the current iteration once exceeded")
    ap.add_argument("--no-balance-topology", dest="balance_topology",
                    action="store_false",
                    help="disable per-epoch oversampling that equalizes the "
                         "Walls/Torus gradient contribution (on by default)")
    ap.add_argument("--conv-channels", type=int, nargs="+", default=[16, 16],
                    metavar="N", help="conv layer output channels (default: 16 16)")
    ap.add_argument("--head-hidden", type=int, nargs="+", default=[24],
                    metavar="N", help="dense-head hidden sizes (default: 24)")
    ap.add_argument("--out", default="training-out/az-conv/az-conv.cnn")
    ap.add_argument("--best-out", default=None,
                    help="path for the best checkpoint; defaults to <out>.best.cnn")
    args = ap.parse_args()

    best_out = args.best_out or (args.out.removesuffix(".cnn") + ".best.cnn")

    th.manual_seed(args.seed)
    np.random.seed(args.seed)
    random.seed(args.seed)
    th.set_num_threads(1)  # parallelism is in Rust self-play, not torch

    # Architecture: conv stack → head-readout ⊕ pool → dense head (7 outputs).
    conv_shapes = []
    prev = IN_CH
    for c in args.conv_channels:
        conv_shapes.append((prev, c))
        prev = c
    head_dims = [2 * prev] + list(args.head_hidden) + [OUTPUTS]
    model = HexConvNet(W, H, "walls", IN_CH, conv_shapes, head_dims)
    gathers = [model.gather_table("walls"), model.gather_table("torus")]
    n_params = sum(p.numel() for p in model.parameters())
    print(f"conv {conv_shapes} head {head_dims} ({n_params} params)")

    export_self_check(model, gathers[0])
    opt = th.optim.Adam(model.parameters(), lr=args.lr)
    buffer = deque(maxlen=args.buffer)
    seed = args.seed * 1_000_000

    best_score = -1.0
    best_iter = -1
    start_time = time.monotonic()
    budget_s = args.max_hours * 3600.0

    for it in range(args.iterations):
        model.eval()
        rows, mean_score, mean_ticks = collect_self_play(cnn_text(model), args, seed)
        seed += args.games_per_iter
        buffer.extend(rows)

        grids = np.array([r[0] for r in buffer], dtype=np.float32).reshape(-1, 3, PLANE)
        n = grids.shape[0]
        planes_np = np.zeros((n, IN_CH, PLANE), dtype=np.float32)
        planes_np[:, :3] = grids
        tid = np.array([r[4] for r in buffer], dtype=np.int64)
        planes_np[:, 3] = (tid == 0)[:, None]  # topology plane: 1 walls, 0 torus
        heads_np = grids[:, 1].argmax(axis=1).astype(np.int64)
        policy_np = np.array([r[1] for r in buffer], dtype=np.float32)
        value_np = np.array([r[2] for r in buffer], dtype=np.float32)
        heading_np = np.array([r[3] for r in buffer], dtype=np.int64)

        planes = th.from_numpy(planes_np)
        heads = th.from_numpy(heads_np)
        policy_t = th.from_numpy(policy_np)
        value_t = th.from_numpy(value_np)
        heading_t = th.from_numpy(heading_np)
        tid_t = th.from_numpy(tid)

        # Balance the gradient across topologies. Torus games last far longer
        # than Walls games (which die fast), so the buffer is dominated by torus
        # states; without balancing the net overfits torus and never learns
        # Walls (run-001: Periodic 77 / Walls ~5). Oversample the minority
        # topology each epoch so both contribute equally many batches.
        subs = [th.nonzero(tid_t == t, as_tuple=False).squeeze(1) for t in (0, 1)]
        target = max(s.numel() for s in subs)

        model.train()
        last = (0.0, 0.0)
        for _ in range(args.epochs):
            # Topology-homogeneous batches so each uses the right gather table.
            for t in (0, 1):
                sub = subs[t]
                if sub.numel() == 0:
                    continue
                if args.balance_topology and sub.numel() < target:
                    extra = sub[th.randint(sub.numel(), (target - sub.numel(),))]
                    perm = th.cat([sub, extra])[th.randperm(target)]
                else:
                    perm = sub[th.randperm(sub.numel())]
                for s in range(0, perm.numel(), args.batch):
                    idx = perm[s : s + args.batch]
                    last = train_step(
                        model, opt, gathers[t],
                        (planes[idx], heads[idx], policy_t[idx], value_t[idx], heading_t[idx]),
                    )

        eval_str = ""
        is_best = False
        if it % args.eval_every == 0 or it == args.iterations - 1:
            model.eval()
            ev_walls, ev_torus, ev_mean, ev_ticks = eval_net(model, args)
            eval_str = (
                f"  eval[W {ev_walls:5.1f} P {ev_torus:5.1f} "
                f"avg {ev_mean:5.1f} t {ev_ticks:5.0f}]"
            )
            is_best = ev_mean > best_score
            if is_best:
                best_score = ev_mean
                best_iter = it
                export_cnn(best_out, model)

        print(
            f"iter {it:3d}  buffer {n:6d}  sp_score {mean_score:5.2f}  "
            f"sp_ticks {mean_ticks:6.1f}  "
            f"ploss {last[0]:.3f}  vloss {last[1]:.3f}"
            + eval_str
            + ("  [best]" if is_best else "")
        )

        if budget_s > 0.0 and time.monotonic() - start_time >= budget_s:
            elapsed_h = (time.monotonic() - start_time) / 3600.0
            print(f"reached --max-hours budget after iter {it} ({elapsed_h:.2f} h)")
            break

    export_cnn(args.out, model)
    print(f"exported policy/value conv net -> {args.out}")
    print(f"best checkpoint (iter {best_iter}, eval_avg {best_score:.2f}) -> {best_out}")


if __name__ == "__main__":
    main()
