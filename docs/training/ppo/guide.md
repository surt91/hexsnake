# Training: PPO (stable-baselines3)

## 1. Überblick

PPO lernt eine Policy über dasselbe **PyO3-Environment** wie DQN
(`crates/snake-py`, `hexsnake_env`): `snake-core` als Gym-artiges Env,
Aktionen = sechs blickrichtungsrelative Richtungen, Beobachtung „rays" = die
20 Sensor-Features.

Der Policy-Kopf (Logits über die sechs Aktionen) wird **1:1 ins
`.mlp`-Format exportiert** und im Spiel pur in Rust/WASM ausgeführt — `argmax`
über die Logits entspricht `argmax` über die MLP-Ausgaben. Trainer:
`python/train_ppo.py` (stable-baselines3 + torch). PPO ist hier meist
stabiler/sample-effizienter als DQN.

> **Hinweis zur eingecheckten Datei**: `crates/snake-core/assets/ppo/policy.mlp`
> ist aktuell ein **Platzhalter** (schwaches GA-Stellvertreter-Netz), damit
> Dropdown-Eintrag „PPO" und Benchmark sofort funktionieren — wird durch den
> echten PPO-Export ersetzt. Echte Läufe macht der Nutzer auf stärkerer
> Hardware.

## 2. Voraussetzungen / Setup

Siehe `python/README.md`:

```bash
cd python
uv venv && source .venv/bin/activate
uv pip install -e '.[train]'
python verify_roundtrip.py        # Gewichtsformat: numpy == Rust?
```

## 3. Smoke-Test (immer zuerst)

```bash
python train_ppo.py --timesteps 50000 --n-envs 4 --out /tmp/ppo-smoke.mlp
```

## 4. Echter Lauf

```bash
python train_ppo.py \
  --timesteps 5000000 \
  --n-envs 8 \
  --boundary walls \
  --max-ticks 2000 \
  --seed 1 \
  --out ppo.mlp
```

- **Architektur** fix auf die In-Game-Topologie:
  `net_arch=dict(pi=[32,24], vf=[32,24])`, `activation_fn=Tanh` — Pflicht,
  damit Policy-Net + Action-Head exakt `[20,32,24,6]` (tanh-Hidden, linear)
  ergeben und der Export passt.
- **Reward-Shaping**: identisch zu DQN (siehe `crates/snake-py/src/env.rs`).
- **Vektorisierung**: `--n-envs` parallele Environments beschleunigen PPO
  deutlich. Mehrere Mio. Steps; auf CPU machbar, mit GPU schneller.

## 5. Auswertung & Einbetten

```bash
cp ppo.mlp ../crates/snake-core/assets/ppo/policy.mlp
cargo test -p snake-core embedded_weights_parse_and_match_architecture
cargo run --release -p snake-core --example benchmark 50 8000   # enthält PPO
```

Im Dropdown als „PPO" aktiv (nativ + WASM).

## 6. CNN-Variante (Vergleich)

`HexSnakeGym(observation="grid")` + `CnnPolicy` für den Vergleich
Sensorstrahlen vs. Brett-Tensor — als Python-Experiment, nicht ins WASM
eingebettet.
