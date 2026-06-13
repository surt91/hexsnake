"""Gymnasium wrapper around the Rust `hexsnake_env.HexSnakeEnv`.

Actions are the six heading-relative directions (0 = straight ahead,
clockwise). Observations are either the 20 sensor "rays" (default, exports
directly to the in-game MLP) or a 3xHxW board "grid" tensor for CNN policies.
"""

from __future__ import annotations

import numpy as np
import gymnasium as gym
from gymnasium import spaces

from . import _native


class HexSnakeGym(gym.Env):
    metadata = {"render_modes": []}

    def __init__(
        self,
        width: int = 16,
        height: int = 12,
        boundary: str = "walls",
        max_ticks: int = 2000,
        observation: str = "rays",
    ):
        super().__init__()
        self._env = _native.HexSnakeEnv(
            width, height, boundary, max_ticks, observation
        )
        self._w, self._h = width, height
        self._obs_kind = observation
        self.action_space = spaces.Discrete(self._env.num_actions)
        if observation == "grid":
            self.observation_space = spaces.Box(
                0.0, 1.0, shape=(3, height, width), dtype=np.float32
            )
        else:
            n = self._env.observation_size
            self.observation_space = spaces.Box(
                -1.0, 1.0, shape=(n,), dtype=np.float32
            )

    def _shape(self, obs):
        arr = np.asarray(obs, dtype=np.float32)
        if self._obs_kind == "grid":
            return arr.reshape(3, self._h, self._w)
        return arr

    def reset(self, *, seed=None, options=None):
        super().reset(seed=seed)
        s = 0 if seed is None else int(seed) & 0xFFFFFFFF
        return self._shape(self._env.reset(s)), {}

    def step(self, action):
        obs, reward, terminated, truncated, score = self._env.step(int(action))
        return self._shape(obs), float(reward), bool(terminated), bool(truncated), {
            "score": score
        }
