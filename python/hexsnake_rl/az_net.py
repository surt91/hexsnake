"""PyTorch policy/value net for gradient-trained AlphaZero-light.

MLP with configurable hidden layers `[21, *hidden, 7]` (tanh hidden, linear
head): outputs 0..6 are the policy logits over the six heading-relative
actions, output 6 is the value. The tanh on the value and the
(reverse-masked) softmax on the policy live on the **Rust** side
(`AlphaZeroLite`), so the exported `.mlp` is just the raw linear layers —
identical layout to every other net in the project. The 21 inputs are the 20
shared sensor features plus the AlphaZero topology bit (`az_features`).

Imports torch, so this module is only pulled in by the training script.
"""

from __future__ import annotations

import torch
import torch.nn as nn

from .export import export_mlp

FEATURE_COUNT = 21  # 20 shared sensor features + 1 topology bit
DEFAULT_HIDDEN = (32, 24)
OUTPUTS = 7
#: Relative-direction index of the (masked) reverse move.
REVERSE = 3
#: The five trainable (non-reverse) action indices.
ACTIONS = [0, 1, 2, 4, 5]


class AZNet(nn.Module):
    def __init__(self, hidden: tuple[int, ...] = DEFAULT_HIDDEN):
        super().__init__()
        dims = (FEATURE_COUNT,) + tuple(hidden) + (OUTPUTS,)
        self._linears = nn.ModuleList(
            nn.Linear(dims[i], dims[i + 1]) for i in range(len(dims) - 1)
        )

    def raw(self, x: torch.Tensor) -> torch.Tensor:
        """The raw 7-dim head output (what the Rust `.mlp` forward returns)."""
        h = x
        for layer in self._linears[:-1]:
            h = torch.tanh(layer(h))
        return self._linears[-1](h)

    def forward(self, x: torch.Tensor):
        out = self.raw(x)
        policy_logits = out[..., :6]
        value = torch.tanh(out[..., 6])
        return policy_logits, value

    def get_dims(self) -> list[int]:
        """Full dims list [input, hidden..., output] derived from layer shapes."""
        dims = [lin.weight.shape[1] for lin in self._linears]
        dims.append(self._linears[-1].weight.shape[0])
        return dims

    def _layers(self):
        return [(lin.weight, lin.bias) for lin in self._linears]

    def to_params(self) -> list[float]:
        """Flat weights in `.mlp` order (per layer: W[out,in] row-major, b)."""
        flat: list[float] = []
        for W, b in self._layers():
            flat.extend(W.detach().cpu().numpy().reshape(-1).tolist())
            flat.extend(b.detach().cpu().numpy().reshape(-1).tolist())
        return flat

    def export(self, path: str) -> None:
        layers = [
            (W.detach().cpu().numpy(), b.detach().cpu().numpy())
            for W, b in self._layers()
        ]
        export_mlp(path, layers)
