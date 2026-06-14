"""PyTorch policy/value net for gradient-trained AlphaZero-light.

One MLP `[20, 32, 24, 7]` (tanh hidden, linear head): outputs 0..6 are the
policy logits over the six heading-relative actions, output 6 is the value.
The tanh on the value and the (reverse-masked) softmax on the policy live on
the **Rust** side (`AlphaZeroLite`), so the exported `.mlp` is just the three
raw linear layers — identical layout to every other net in the project.

Imports torch, so this module is only pulled in by the training script.
"""

from __future__ import annotations

import torch
import torch.nn as nn

from .export import export_mlp

FEATURE_COUNT = 20
HIDDEN = (32, 24)
OUTPUTS = 7
#: Relative-direction index of the (masked) reverse move.
REVERSE = 3
#: The five trainable (non-reverse) action indices.
ACTIONS = [0, 1, 2, 4, 5]


class AZNet(nn.Module):
    def __init__(self):
        super().__init__()
        self.l1 = nn.Linear(FEATURE_COUNT, HIDDEN[0])
        self.l2 = nn.Linear(HIDDEN[0], HIDDEN[1])
        self.head = nn.Linear(HIDDEN[1], OUTPUTS)

    def raw(self, x: torch.Tensor) -> torch.Tensor:
        """The raw 7-dim head output (what the Rust `.mlp` forward returns)."""
        h = torch.tanh(self.l1(x))
        h = torch.tanh(self.l2(h))
        return self.head(h)

    def forward(self, x: torch.Tensor):
        out = self.raw(x)
        policy_logits = out[..., :6]
        value = torch.tanh(out[..., 6])
        return policy_logits, value

    def _layers(self):
        return [
            (self.l1.weight, self.l1.bias),
            (self.l2.weight, self.l2.bias),
            (self.head.weight, self.head.bias),
        ]

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
