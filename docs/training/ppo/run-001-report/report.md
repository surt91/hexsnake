# Training Report: PPO — Run 001

**Datum**: 2026-06-14  
**Ziel**: Erster produktiver PPO-Lauf mit gemischten Rändern.
Baseline bisher: Smoke-Run (Walls Ø 35, Periodic Ø 58).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | PPO (stable-baselines3) |
| `--timesteps` | 5 000 000 |
| `--n-envs` | 8 |
| `--boundary` | mixed (50/50 Walls/Torus) |
| `--max-ticks` | 2 000 |
| `--seed` | 1 |
| Architektur | 20→32→24→6 (pi+vf net_arch=[32,24], Tanh) |
| Reward | +1 Fressen, −1 Tod, ±0.1×Δdist, −0.005/Tick |

**Begründung**: PPO ist sample-effizienter als DQN durch On-Policy-Updates
und Vektorisierung (8 parallele Environments). Mit 5M Steps sollte das Netz
deutlich über Greedy-Niveau kommen.

---

## 2. Lernkurve

_Aus TensorBoard/Logging nach Abschluss._

| Steps (k) | ep_rew_mean | Bemerkung |
|---|---|---|
| 1 000 | _todo_ | |
| 2 000 | _todo_ | |
| 3 000 | _todo_ | |
| 5 000 | _todo_ | |

---

## 3. Benchmark-Ergebnis

| Topologie  | Smoke-Run | **Run 001** | Δ |
|------------|----------:|------------:|--:|
| Walls Ø    |     34.70 |      _todo_ | _todo_ |
| Periodic Ø |     58.14 |      _todo_ | _todo_ |

---

## 4. Vergleich mit anderen Methoden

| Methode     | Walls Ø | Periodic Ø |
|-------------|--------:|-----------:|
| MLP-GA 005  |   91.40 |     125.12 |
| NEAT run-001| _todo_  |     _todo_ |
| DQN run-001 | _todo_  |     _todo_ |
| PPO run-001 | _todo_  |     _todo_ |
| AZ run-001  | _todo_  |     _todo_ |

---

## 5. Beobachtungen

_Wird nach Abschluss ergänzt._

---

## 6. Fazit

_PPO vs. DQN: Zeigt sich die höhere Sample-Effizienz in besseren Scores?_  
_Wie nähert sich PPO dem MLP-GA an?_

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/ppo/policy.mlp` | Eingecheckt nach Deployment |
| `training-out/ppo-run-001/best.mlp` | Bestes Netz |
