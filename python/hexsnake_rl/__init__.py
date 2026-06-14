"""HexSnake reinforcement-learning helpers.

Wraps the Rust `hexsnake_env` extension (PyO3 over `snake-core`) as a
Gymnasium environment and provides the weight exporter that writes the
in-game `.mlp` format.
"""

from . import _native
from .gym_env import HexSnakeGym
from .export import export_mlp, from_ppo, from_dqn

#: Run `snake-core`'s MLP forward pass on a `.mlp` text and an input vector.
mlp_forward = _native.mlp_forward
#: Play one AlphaZero-light self-play game in Rust (GIL released).
az_selfplay = _native.az_selfplay

__all__ = [
    "HexSnakeGym",
    "export_mlp",
    "from_ppo",
    "from_dqn",
    "mlp_forward",
    "az_selfplay",
]
