"""Train a DQN policy against the HexSnake env and export it to `.mlp`.

Requires the `train` extra (stable-baselines3 + torch). `argmax` over the
Q-values equals `argmax` over the in-game MLP outputs, so the Q-network
exports directly into the same inference path.

Example:
    python train_dqn.py --timesteps 2000000 --out dqn.mlp
"""

import argparse

import torch
from stable_baselines3 import DQN

from hexsnake_rl import HexSnakeGym, export_mlp, from_dqn


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--timesteps", type=int, default=2_000_000)
    ap.add_argument("--width", type=int, default=16)
    ap.add_argument("--height", type=int, default=12)
    ap.add_argument("--boundary", default="walls", choices=["walls", "torus"])
    ap.add_argument("--max-ticks", type=int, default=2000)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--out", default="dqn.mlp")
    args = ap.parse_args()

    env = HexSnakeGym(args.width, args.height, args.boundary, args.max_ticks)
    model = DQN(
        "MlpPolicy",
        env,
        verbose=1,
        seed=args.seed,
        learning_starts=10_000,
        buffer_size=200_000,
        # tanh hidden to match the Rust forward pass; [32, 24] like the MLP.
        policy_kwargs=dict(activation_fn=torch.nn.Tanh, net_arch=[32, 24]),
    )
    model.learn(total_timesteps=args.timesteps)
    export_mlp(args.out, from_dqn(model))
    print(f"exported policy -> {args.out}")


if __name__ == "__main__":
    main()
