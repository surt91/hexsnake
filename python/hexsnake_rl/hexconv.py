"""PyTorch hex-conv net that mirrors `snake-core`'s `HexConv` exactly.

The in-game inference is a pure-Rust hex convolution (7-tap: center + 6 hex
neighbors), followed by a head-cell readout concatenated with a global average
pool, fed to a dense head (tanh hidden, linear output). This module reproduces
that forward pass so a net trained here exports straight into the `.cnn`
format and plays identically in Rust/WASM.

The neighbor geometry is taken from the Rust side (`neighbor_table`) instead of
re-deriving the hex math in Python, so walls (zero-pad) and torus (wrap) match
the engine bit-for-bit. A model is built for one board size (the gather table
is fixed); the exported weights are size-agnostic because Rust rebuilds the
table for whatever board it runs on.

Imports torch, so it is only pulled in by the training scripts, not by
`hexsnake_rl.__init__`.
"""

from __future__ import annotations

from typing import Sequence

import torch as th
import torch.nn as nn

from . import _native

TAPS = 7  # center + 6 neighbors


def _gather_index(width: int, height: int, boundary: str) -> th.Tensor:
    """`(N, 7)` long tensor: tap 0 = the cell itself, taps 1..6 = its hex
    neighbors. Off-board neighbors map to index `N` (a zero-padding slot)."""
    n = width * height
    table = _native.neighbor_table(width, height, boundary)  # flat N*6, -1 = off
    idx = th.full((n, TAPS), n, dtype=th.long)  # default = pad slot
    idx[:, 0] = th.arange(n)
    for cell in range(n):
        for t in range(6):
            nb = table[cell * 6 + t]
            if nb >= 0:
                idx[cell, t + 1] = nb
    return idx


class HexConvNet(nn.Module):
    """Conv stack → head-cell readout ⊕ global pool → dense head.

    `conv_shapes` is a list of `(in_ch, out_ch)`; the first `in_ch` must equal
    the number of input planes. `head_dims` are the dense head dims, the first
    of which must be `2 * last_conv_out_ch` and the last the output count
    (6 for the standalone policy, 7 for AlphaZero's policy+value).
    """

    def __init__(
        self,
        width: int,
        height: int,
        boundary: str,
        in_channels: int,
        conv_shapes: Sequence[tuple[int, int]],
        head_dims: Sequence[int],
    ):
        super().__init__()
        self.width = width
        self.height = height
        self.in_channels = in_channels
        self.conv_shapes = [tuple(s) for s in conv_shapes]
        self.head_dims = list(head_dims)

        last_out = conv_shapes[-1][1] if conv_shapes else in_channels
        assert head_dims[0] == 2 * last_out, "head input must be 2×last_conv_out_ch"

        self.convs = nn.ParameterList()
        self.conv_bias = nn.ParameterList()
        for i, o in conv_shapes:
            # Weight layout [o][i][tap] matches the Rust `.cnn` flat order.
            self.convs.append(nn.Parameter(th.empty(o, i, TAPS).normal_(0, 0.3)))
            self.conv_bias.append(nn.Parameter(th.zeros(o)))

        head = []
        for a, b in zip(head_dims, head_dims[1:]):
            head.append(nn.Linear(a, b))
        self.head = nn.ModuleList(head)

        self.register_buffer("gather", _gather_index(width, height, boundary))

    def forward(self, planes: th.Tensor, head_idx: th.Tensor) -> th.Tensor:
        """`planes`: `(B, in_channels, N)`; `head_idx`: `(B,)` cell indices."""
        b = planes.shape[0]
        x = planes
        for w, bias in zip(self.convs, self.conv_bias):
            pad = th.zeros(b, x.shape[1], 1, dtype=x.dtype, device=x.device)
            xp = th.cat([x, pad], dim=2)  # (B, in, N+1)
            gathered = xp[:, :, self.gather]  # (B, in, N, 7)
            # out[b,o,n] = bias[o] + Σ_{i,t} W[o,i,t] · gathered[b,i,n,t]
            x = th.einsum("oit,bint->bon", w, gathered) + bias[None, :, None]
            x = th.tanh(x)

        # Head-cell readout ⊕ global average pool.
        readout = x[th.arange(b), :, head_idx]  # (B, C)
        pooled = x.mean(dim=2)  # (B, C)
        feat = th.cat([readout, pooled], dim=1)  # (B, 2C)
        for i, lin in enumerate(self.head):
            feat = lin(feat)
            if i < len(self.head) - 1:
                feat = th.tanh(feat)
        return feat
