"""Behavior-cloning trainer for the hex-conv ConvNet.

Clones the strongest classical AI — the A* `PathPlanner` (+ tail-check) — into a
`HexConvNet`, then exports the `.cnn` weights. Expert states come from the Rust
`expert_rollout` binding (pure Rust, GIL released), labelled with the planner's
chosen **absolute** move; the ConvNet emits absolute direction scores, so the
label is just the move's index — no heading conversion.

Mixed topologies are handled correctly: the hex convolution's neighbor geometry
differs between walls (zero-pad) and torus (wrap), so each sample is fed through
the gather table matching its board.

Run:  python train_cnn.py --games 1500 --epochs 30 --out training-out/cnn/best.cnn
"""

from __future__ import annotations

import argparse

import numpy as np
import torch as th
import torch.nn as nn

from hexsnake_rl import _native
from hexsnake_rl.export import export_cnn
from hexsnake_rl.hexconv import HexConvNet

W, H, IN_CH = 16, 12, 4
PLANE = W * H
BOUNDARIES = ["walls", "torus"]


def collect(games: int, max_ticks: int, seed: int):
    """Gather expert states for both topologies. Returns planes (N,4,PLANE),
    head indices (N,), labels (N,), topology id (N,) (0=walls, 1=torus) and a
    food-approach flag (N,) (1.0 if the expert move advanced toward food)."""
    planes_all, heads_all, labels_all, topo_all, toward_all = [], [], [], [], []
    for tid, boundary in enumerate(BOUNDARIES):
        grids, labels, toward = _native.expert_rollout(boundary, W, H, max_ticks, games, seed)
        grids = np.asarray(grids, dtype=np.float32).reshape(-1, 3, PLANE)
        n = grids.shape[0]
        planes = np.zeros((n, IN_CH, PLANE), dtype=np.float32)
        planes[:, :3] = grids
        planes[:, 3] = 1.0 if boundary == "walls" else 0.0  # topology plane
        heads = grids[:, 1].argmax(axis=1)  # head channel
        planes_all.append(planes)
        heads_all.append(heads)
        labels_all.append(np.asarray(labels, dtype=np.int64))
        topo_all.append(np.full(n, tid, dtype=np.int64))
        toward_all.append(np.asarray(toward, dtype=np.float32))
        print(f"  {boundary}: {n} expert states ({np.mean(toward):.0%} toward food)")
    return (
        np.concatenate(planes_all),
        np.concatenate(heads_all),
        np.concatenate(labels_all),
        np.concatenate(topo_all),
        np.concatenate(toward_all),
    )


def run_epoch(model, gathers, planes, heads, labels, topo, weights, opt, loss_fn, bs, train):
    """One pass. Batches are topology-homogeneous so each uses the right gather
    table. Per-sample `weights` upweight food-seeking moves. Returns (mean loss,
    accuracy)."""
    idx = np.arange(len(labels))
    if train:
        np.random.shuffle(idx)
    total_loss, correct, seen = 0.0, 0, 0
    for tid in (0, 1):
        sub = idx[topo[idx] == tid]
        gather = gathers[tid]
        for i in range(0, len(sub), bs):
            b = sub[i : i + bs]
            p = th.from_numpy(planes[b])
            hd = th.from_numpy(heads[b])
            y = th.from_numpy(labels[b])
            wt = th.from_numpy(weights[b])
            with th.set_grad_enabled(train):
                logits = model(p, hd, gather)
                loss = (loss_fn(logits, y) * wt).sum() / wt.sum()
                if train:
                    opt.zero_grad()
                    loss.backward()
                    opt.step()
            total_loss += loss.item() * len(b)
            correct += int((logits.argmax(1) == y).sum())
            seen += len(b)
    return total_loss / seen, correct / seen


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--games", type=int, default=1500, help="games per topology")
    ap.add_argument("--epochs", type=int, default=30)
    ap.add_argument("--max-ticks", type=int, default=2000)
    ap.add_argument("--batch", type=int, default=256)
    ap.add_argument("--lr", type=float, default=1e-3)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--toward-weight", type=float, default=4.0,
                    help="loss weight multiplier for moves that advance toward food")
    ap.add_argument("--out", default="training-out/cnn/best.cnn")
    args = ap.parse_args()

    th.manual_seed(args.seed)
    np.random.seed(args.seed)

    print(f"collecting expert data ({args.games} games × 2 topologies)…")
    planes, heads, labels, topo, toward = collect(args.games, args.max_ticks, args.seed)
    n = len(labels)
    # Upweight food-seeking moves so the clone commits to food instead of
    # collapsing into safe circling (the documented failure mode).
    weights = (1.0 + (args.toward_weight - 1.0) * toward).astype(np.float32)
    print(f"total {n} states, toward-weight {args.toward_weight}")

    # Train/val split.
    perm = np.random.permutation(n)
    n_val = n // 10
    val, tr = perm[:n_val], perm[n_val:]

    model = HexConvNet(W, H, "walls", IN_CH, [(4, 16), (16, 16)], [32, 24, 6])
    gathers = [model.gather_table("walls"), model.gather_table("torus")]
    opt = th.optim.Adam(model.parameters(), lr=args.lr)
    loss_fn = nn.CrossEntropyLoss(reduction="none")

    best_val = 0.0
    best_state = None
    for e in range(args.epochs):
        model.train()
        tl, ta = run_epoch(
            model, gathers, planes[tr], heads[tr], labels[tr], topo[tr], weights[tr],
            opt, loss_fn, args.batch, True,
        )
        model.eval()
        vl, va = run_epoch(
            model, gathers, planes[val], heads[val], labels[val], topo[val], weights[val],
            opt, loss_fn, args.batch, False,
        )
        flag = ""
        if va > best_val:
            best_val = va
            best_state = {k: v.detach().clone() for k, v in model.state_dict().items()}
            flag = " *"
        print(f"epoch {e:2d}: train loss {tl:.3f} acc {ta:.3f} | val loss {vl:.3f} acc {va:.3f}{flag}")

    if best_state is not None:
        model.load_state_dict(best_state)
    print(f"best val accuracy: {best_val:.3f}")
    export_cnn(args.out, model)
    print(f"exported {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
