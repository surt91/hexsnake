# Training Report: PPO — Run 002

**Datum**: 2026-06-14  
**Ziel**: Mehr Budget (10M Schritte) und stärkerer Food-Reward.
Baseline: Run 001 (Walls Ø 38.24, Periodic Ø 58.62).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | PPO (stable-baselines3) |
| `--timesteps` | 10 000 000 |
| `--n-envs` | 8 |
| `--boundary` | mixed (50/50 Walls/Torus) |
| `--max-ticks` | 2 000 |
| `--seed` | 2 |
| Architektur | 20→32→24→6 (pi+vf net_arch=[32,24], Tanh) |
| GPU | CUDA (nvidia) |

**Reward-Änderungen gegenüber Run 001:**

| Konstante | Run 001 | Run 002 |
|---|---|---|
| `REWARD_EAT` | 1.0 | **3.0** |
| `REWARD_STEP` | −0.005 | **−0.01** |
| `REWARD_DEATH` | −1.0 | −1.0 |
| `REWARD_APPROACH` | 0.1 | 0.1 |

---

## 2. Lernkurve

| Steps (k) | explained_var | entropy_loss | Bemerkung |
|---|---|---|---|
| 32      | −1.78 | 0.453 | fast zufällig |
| 344     | −0.745 | 0.869 | schneller Anstieg |
| 1 000   | −0.530 | 0.944 | |
| 2 000   | −0.443 | 0.955 | |
| 5 000   | −0.371 | 0.951 | |
| 10 000  | −0.190 | 0.949 | Endwert |

`explained_variance` bleibt bei ~0.95 ab ca. 1M Schritten stabil.
`entropy_loss` sinkt von −1.78 auf −0.19 — Policy wird deterministischer
als Run 001 (der bei −0.25 endete).

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks.

| Topologie  | Run 001 | **Run 002** | Δ |
|------------|--------:|------------:|--:|
| Walls Ø    |   38.24 |   **34.46** | −9.9 % |
| Periodic Ø |   58.62 |   **57.76** | −1.5 % |

**Leichte Regression gegenüber Run 001.** Run 002 wird nicht als
`policy.mlp` eingecheckt — Run 001 bleibt deployed.

---

## 4. Vergleich mit anderen Methoden

| Methode       | Walls Ø | Periodic Ø |
|---------------|--------:|-----------:|
| MLP-GA 005    |   91.40 |     125.12 |
| NEAT run-001  |   38.92 |      56.84 |
| **PPO run-001**| **38.24** | **58.62** |
| PPO run-002   |   34.46 |      57.76 |
| AZ run-001    |   37.60 |      54.28 |
| DQN run-002   |   31.70 |      51.70 |

---

## 5. Analyse

### 5.1 Warum Regression trotz 2× Budget?

- **Entropy zu niedrig**: entropy_loss −0.19 vs −0.25 in Run 001.
  Die Policy ist deterministischer — weniger Exploration, potenzielle
  Überanpassung an die gemischte Trainingsverteilung.

- **Reward-Imbalanz**: `REWARD_EAT=3.0` + `REWARD_STEP=−0.01` erhöht
  den Druck, sofort Futter zu essen. Die Policy lernt aggressiveres
  Fressverhalten (avg Ticks 421 vs 436 in Run 001 — die Schlange lebt
  kürzer). Auf Walls ist Aggressivität riskanter als auf Torus.

- **5M vs 10M Steps**: Ab ca. 1M Schritten ist die Konvergenz eingetreten
  (explained_variance 0.94+). Weitere Steps verfeinern die Policy ohne
  klaren Richtungswechsel. Das Budget war kein Engpass.

### 5.2 Vergleich mit DQN run-002

DQN (off-policy, walls-only) profitiert stärker vom neuen Reward als PPO
(on-policy, mixed): DQN +38.1 % Walls vs PPO −9.9 % Walls. Möglicher
Grund: DQN-Replay schützt vor kurzzeitigem Verhalten durch Erfahrungsaggregation,
während PPO-On-Policy-Updates direkt auf aggressivere Samples reagieren.

---

## 6. Schlussfolgerungen für Run 003

- Original Reward beibehalten (`REWARD_EAT=1.0`, `REWARD_STEP=−0.005`)
- Walls-only Training testen (wie DQN run-002 → +38 % Walls)
- 5M Steps reichen (Konvergenz bei ~1M, 10M bringt keinen Vorteil)
- Alternativ: nur `REWARD_EAT` erhöhen, `REWARD_STEP` unverändert lassen

---

## 7. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/ppo-run-002/policy.mlp` | Gewichte Run 002 (nicht deployed) |
| `training-out/ppo-run-002/train.log` | Training-Logs (SB3-Output) |
| `crates/snake-core/assets/ppo/policy.mlp` | **Run 001** bleibt embedded |
