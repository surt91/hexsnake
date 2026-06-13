"""A small CNN features extractor for the 3xHxW board observation.

Used with stable-baselines3 ``CnnPolicy`` when training on the ``grid``
observation. The default SB3 ``NatureCNN`` assumes large images; this one is
sized for the small hex-board tensor. Imports torch, so it is only pulled in
by the training scripts (the `train` extra), not by `hexsnake_rl.__init__`.
"""

from __future__ import annotations

import torch as th
import torch.nn as nn
from gymnasium import spaces
from stable_baselines3.common.torch_layers import BaseFeaturesExtractor


class SmallGridCNN(BaseFeaturesExtractor):
    def __init__(self, observation_space: spaces.Box, features_dim: int = 64):
        super().__init__(observation_space, features_dim)
        channels = observation_space.shape[0]  # 3: body / head / food
        self.cnn = nn.Sequential(
            nn.Conv2d(channels, 16, kernel_size=3, padding=1),
            nn.ReLU(),
            nn.Conv2d(16, 32, kernel_size=3, padding=1),
            nn.ReLU(),
            nn.Flatten(),
        )
        with th.no_grad():
            n_flat = self.cnn(th.zeros(1, *observation_space.shape)).shape[1]
        self.linear = nn.Sequential(nn.Linear(n_flat, features_dim), nn.ReLU())

    def forward(self, observations: th.Tensor) -> th.Tensor:
        return self.linear(self.cnn(observations))
