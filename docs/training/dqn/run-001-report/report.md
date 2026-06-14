# Training Report: DQN — Run 001

**Datum**: 2026-06-14  
**Ziel**: Erster produktiver DQN-Lauf mit gemischten Rändern.
Baseline bisher: Smoke-Run (Walls Ø 28, Periodic Ø 48).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | DQN (stable-baselines3) |
| `--timesteps` | 3 000 000 |
| `--boundary` | mixed (50/50 Walls/Torus) |
| `--max-ticks` | 2 000 |
| `--seed` | 1 |
| Architektur | 20→32→24→6 (net_arch=[32,24], Tanh) |
| Reward | +1 Fressen, −1 Tod, ±0.1×Δdist, −0.005/Tick |

**Begründung**: DQN ist sample-ineffizienter als PPO, aber konzeptuell
einfacher (keine Value-Function-Approximation beim Policy-Update). Der
Vergleich mit PPO zeigt, ob das in dieser Umgebung relevant ist.

---

## 2. Lernkurve

_Aus TensorBoard/Logging nach Abschluss._

| Steps (k) | ep_rew_mean | Bemerkung |
|---|---|---|
| 500   | _todo_ | |
| 1 000 | _todo_ | |
| 2 000 | _todo_ | |
| 3 000 | _todo_ | |

---

## 3. Benchmark-Ergebnis

| Topologie  | Smoke-Run | **Run 001** | Δ |
|------------|----------:|------------:|--:|
| Walls Ø    |     28.18 |      _todo_ | _todo_ |
| Periodic Ø |     48.08 |      _todo_ | _todo_ |

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

_Wie schlägt sich DQN vs. PPO und vs. dem evolutionären MLP-GA?_

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/dqn/policy.mlp` | Eingecheckt nach Deployment |
| `training-out/dqn-run-001/best.mlp` | Bestes Netz |
