# HexSnake — Python RL track (DQN / PPO)

`snake-core` exposed to Python via PyO3/maturin as a Gym-like environment, so
stable-baselines3 can train DQN and PPO against the *exact* deterministic game
the app and server use. Trained policies are exported back into the in-game
`.mlp` weight format — **inference stays pure Rust/WASM**, Python is only used
for training.

## Setup (uv)

```bash
cd python
uv venv
source .venv/bin/activate

# Build + install the Rust extension and the env deps (numpy, gymnasium):
uv pip install -e .

# Optional: the heavy RL stack (stable-baselines3 + torch) for training:
uv pip install -e '.[train]'
```

`maturin` (the build backend) compiles `crates/snake-py` with the `python`
feature and installs the `hexsnake_env` extension module.

## Verify the weight roundtrip (no torch needed)

```bash
python verify_roundtrip.py
```

Builds a random MLP in numpy, exports it, and checks the numpy forward pass
against the **actual Rust inference** (`hexsnake_rl.mlp_forward`). This
guards the Python→Rust weight layout — the classic transposition trap.

## Smoke-test the environment

```bash
python -c "from hexsnake_rl import HexSnakeGym; e=HexSnakeGym(); o,_=e.reset(seed=0); \
print('obs', len(o)); print(e.step(0))"
```

## Train

```bash
# PPO (rays observation, matches the in-game MLP topology):
python train_ppo.py --timesteps 2000000 --n-envs 8 --out ppo.mlp

# DQN:
python train_dqn.py --timesteps 2000000 --out dqn.mlp
```

Reward shaping (in `crates/snake-py/src/env.rs`): +1 eating, +2 winning,
−1 dying, ±0.1 × change in (torus-aware) food distance, −0.005 per step.

## Observations

- `observation="rays"` (default): the 20 heading-relative sensor features —
  the same vector the in-game MLP/NEAT use, so the export is 1:1.
- `observation="grid"`: a 3×H×W board tensor (body / head / food) for **CNN**
  policies (`CnnPolicy`). Comparison experiment; CNN policies are evaluated in
  Python and are not embedded into the WASM build (pure-Rust conv inference is
  out of scope).

## Embed a trained policy

```bash
cp ppo.mlp ../crates/snake-core/assets/ppo/policy.mlp
cp dqn.mlp ../crates/snake-core/assets/dqn/policy.mlp
cargo test -p snake-core embedded            # parses & plays legally?
cargo run --release -p snake-core --example benchmark 30 5000
```

The checked-in `assets/{dqn,ppo}/policy.mlp` are untrained placeholders until
you run the real training. See `docs/training/dqn.md` and
`docs/training/ppo.md`.
