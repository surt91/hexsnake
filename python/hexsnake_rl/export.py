"""Export a trained policy to the in-game `.mlp` weight format.

The format (see `crates/snake-core/src/nn/mlp.rs`) is, per layer, the weight
matrix row-major (out x in) followed by the bias vector; layers in order;
hidden layers use tanh, the output layer is linear. A PyTorch ``nn.Linear``
stores its weight as ``(out, in)`` row-major too, so a faithful export is just
a concatenation — the tricky part is getting the *order* right, which
``verify_roundtrip.py`` checks against the actual Rust forward pass.
"""

from __future__ import annotations

from typing import Sequence, Tuple

import numpy as np

MAGIC = "hexsnake-mlp v1"

Layer = Tuple[np.ndarray, np.ndarray]  # (weight [out, in], bias [out])


def export_mlp(path: str, layers: Sequence[Layer]) -> None:
    """Write `layers` to `path` in the `.mlp` text format.

    Each layer is `(W, b)` with `W.shape == (out, in)` and `b.shape == (out,)`.
    The activation between hidden layers (tanh) and the final linear output are
    implied by position, exactly as the Rust side reconstructs them.
    """
    dims = [int(layers[0][0].shape[1])] + [int(W.shape[0]) for W, _ in layers]
    params = []
    for W, b in layers:
        W = np.asarray(W, dtype=np.float32)
        b = np.asarray(b, dtype=np.float32)
        assert b.shape[0] == W.shape[0], "bias/weight out-dim mismatch"
        params.extend(W.reshape(-1).tolist())  # row-major (out x in)
        params.extend(b.reshape(-1).tolist())

    lines = [MAGIC, " ".join(str(d) for d in dims)]
    # 16 params per line for readability; float() repr round-trips f32 exactly.
    for i in range(0, len(params), 16):
        lines.append(" ".join(repr(float(p)) for p in params[i : i + 16]))
    import os
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")


def _linear_layers(module) -> list[Layer]:
    """Collect (W, b) from every nn.Linear inside a torch Sequential, in order."""
    import torch.nn as nn

    out: list[Layer] = []
    for m in module.modules():
        if isinstance(m, nn.Linear):
            out.append(
                (m.weight.detach().cpu().numpy(), m.bias.detach().cpu().numpy())
            )
    return out


def from_ppo(model) -> list[Layer]:
    """Layers of an SB3 PPO MlpPolicy: shared/policy net + the action head.

    Train PPO with ``policy_kwargs=dict(activation_fn=torch.nn.Tanh,
    net_arch=dict(pi=[32, 24], vf=[32, 24]))`` so the topology matches the
    in-game MLP ([20, 32, 24, 6], tanh hidden, linear out).
    """
    policy = model.policy
    layers = _linear_layers(policy.mlp_extractor.policy_net)
    layers.append(
        (
            policy.action_net.weight.detach().cpu().numpy(),
            policy.action_net.bias.detach().cpu().numpy(),
        )
    )
    return layers


def from_dqn(model) -> list[Layer]:
    """Layers of an SB3 DQN policy's Q-network (argmax over Q == argmax over
    the MLP outputs). Train DQN with
    ``policy_kwargs=dict(activation_fn=torch.nn.Tanh, net_arch=[32, 24])`` so
    the hidden activations match the Rust tanh."""
    return _linear_layers(model.policy.q_net.q_net)
