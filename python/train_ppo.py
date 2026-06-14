"""Train a PPO policy against the HexSnake env and export it to `.mlp`.

Requires the `train` extra (stable-baselines3 + torch). The exported network
matches the in-game MLP ([20, 32, 24, 6], tanh hidden, linear output) and can
be dropped into `crates/snake-core/assets/ppo/policy.mlp`.

Example:
    python train_ppo.py --timesteps 2000000 --n-envs 8 --out ppo.mlp
"""

import argparse

import torch
from stable_baselines3 import PPO
from stable_baselines3.common.vec_env import DummyVecEnv

from hexsnake_rl import HexSnakeGym, export_mlp, from_ppo


def make_env(args, rank):
    def thunk():
        return HexSnakeGym(
            args.width, args.height, args.boundary, args.max_ticks, args.observation
        )

    return thunk


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--timesteps", type=int, default=2_000_000)
    ap.add_argument("--n-envs", type=int, default=8)
    ap.add_argument("--width", type=int, default=16)
    ap.add_argument("--height", type=int, default=12)
    ap.add_argument("--boundary", default="walls", choices=["walls", "torus", "mixed"])
    ap.add_argument("--max-ticks", type=int, default=2000)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument(
        "--observation",
        default="rays",
        choices=["rays", "grid"],
        help="rays exports to the in-game MLP; grid trains a CNN (no export)",
    )
    ap.add_argument("--out", default="ppo.mlp")
    args = ap.parse_args()

    env = DummyVecEnv([make_env(args, i) for i in range(args.n_envs)])
    if args.observation == "grid":
        # CNN comparison run: not embeddable in WASM, so save the SB3 model
        # instead of exporting the .mlp format.
        from hexsnake_rl.cnn import SmallGridCNN

        model = PPO(
            "CnnPolicy",
            env,
            verbose=1,
            seed=args.seed,
            policy_kwargs=dict(
                activation_fn=torch.nn.Tanh,
                net_arch=[64],
                features_extractor_class=SmallGridCNN,
                features_extractor_kwargs=dict(features_dim=64),
            ),
        )
        model.learn(total_timesteps=args.timesteps)
        model.save(args.out)
        print(f"saved CNN model -> {args.out}.zip (not embeddable in WASM)")
        return

    model = PPO(
        "MlpPolicy",
        env,
        verbose=1,
        seed=args.seed,
        # Match the in-game MLP exactly so the export is a 1:1 mapping.
        policy_kwargs=dict(
            activation_fn=torch.nn.Tanh,
            net_arch=dict(pi=[32, 24], vf=[32, 24]),
        ),
    )
    model.learn(total_timesteps=args.timesteps)
    export_mlp(args.out, from_ppo(model))
    print(f"exported policy -> {args.out}")


if __name__ == "__main__":
    main()
