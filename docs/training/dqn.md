# Training: DQN (stable-baselines3)

## 1. Überblick

DQN lernt eine Q-Funktion über das **PyO3-Environment** (`crates/snake-py`,
`hexsnake_env`), das `snake-core` als Gym-artiges Env nach Python bringt
(`reset`/`step`/Observation). Aktionen sind die sechs blickrichtungsrelativen
Richtungen (0 = geradeaus, im Uhrzeigersinn) — identisch zur Output-Konvention
des In-Game-MLP. Beobachtung „rays" = die 20 Sensor-Features (wie MLP/NEAT).

`argmax` über die Q-Werte entspricht `argmax` über die MLP-Ausgaben; das
Q-Netz wird daher **1:1 ins `.mlp`-Format exportiert** und im Spiel pur in
Rust/WASM ausgeführt (kein PyTorch zur Laufzeit). Trainer: `python/train_dqn.py`
(stable-baselines3 + torch).

> **Hinweis zur eingecheckten Datei**: `crates/snake-core/assets/dqn/policy.mlp`
> ist aktuell ein **Platzhalter** (ein schwaches GA-Stellvertreter-Netz),
> damit der Dropdown-Eintrag „DQN" und der Benchmark sofort funktionieren. Er
> wird durch den echten DQN-Export ersetzt, sobald du trainiert hast. Echte
> RL-Läufe macht der Nutzer auf stärkerer Hardware (GPU empfohlen).

## 2. Voraussetzungen / Setup

Siehe `python/README.md`. Kurzform (mit `uv`):

```bash
cd python
uv venv && source .venv/bin/activate
uv pip install -e '.[train]'     # baut die Rust-Extension + SB3/torch
```

Roundtrip des Gewichtsformats prüfen (ohne torch):

```bash
python verify_roundtrip.py       # numpy-Export == Rust-Inferenz?
```

## 3. Smoke-Test (immer zuerst)

```bash
python -c "from hexsnake_rl import HexSnakeGym; e=HexSnakeGym(); \
o,_=e.reset(seed=0); print(len(o), e.step(0))"
python train_dqn.py --timesteps 20000 --out /tmp/dqn-smoke.mlp
```

Verifiziert Env, Training-Loop und Export in Minuten.

## 4. Echter Lauf

```bash
python train_dqn.py \
  --timesteps 3000000 \
  --boundary walls \
  --max-ticks 2000 \
  --seed 1 \
  --out dqn.mlp
```

- **Architektur** ist fix auf die In-Game-Topologie gesetzt
  (`net_arch=[32, 24]`, `activation_fn=Tanh`) — Pflicht, sonst passt der
  Export nicht zur Rust-Inferenz (tanh-Hidden, lineare Ausgabe).
- **Reward-Shaping** (in `crates/snake-py/src/env.rs`): +1 Fressen,
  +2 Gewinnen, −1 Tod, ±0,1 × Änderung der (Torus-)Distanz zum Futter,
  −0,005 pro Schritt.
- **Laufzeit**: DQN ist sample-ineffizient; mehrere Stunden auf CPU,
  deutlich schneller mit GPU. Mit `--boundary torus` separat trainieren
  oder beide Ränder vergleichen.

## 5. Auswertung & Einbetten

```bash
cp dqn.mlp ../crates/snake-core/assets/dqn/policy.mlp
cargo test -p snake-core embedded_weights_parse_and_match_architecture
cargo run --release -p snake-core --example benchmark 50 8000   # enthält DQN
```

Danach ist das Netz im Dropdown „DQN" aktiv (nativ und WASM). Asset-Wechsel
im selben Commit wie etwaige Format-/Trainer-Änderungen pflegen.

## 6. CNN-Variante (Vergleich)

Mit `HexSnakeGym(observation="grid")` liefert das Env einen 3×H×W-Brett-Tensor
(Körper/Kopf/Futter) für eine `CnnPolicy`. Das dient dem Vergleich
Sensorstrahlen vs. CNN **in Python**; CNN-Policies werden nicht ins WASM
eingebettet (pure-Rust-Conv-Inferenz ist außerhalb des Scopes).
