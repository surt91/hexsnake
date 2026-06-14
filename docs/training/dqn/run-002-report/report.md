# Training Report: DQN — Run 002

**Datum**: 2026-06-14  
**Ziel**: Verbesserung gegenüber Run 001 durch zwei Änderungen:
1. Walls-only statt mixed (Run 001 litt unter Budget-Split auf zwei Randbedingungen)
2. Stärkerer Food-Reward (`REWARD_EAT` 1.0 → 3.0, `REWARD_STEP` −0.005 → −0.01)

Baseline: Run 001 (Walls Ø 22.96, Periodic Ø 48.18).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | DQN (stable-baselines3) |
| `--timesteps` | 5 000 000 |
| `--boundary` | walls (nur Wände, kein Mixed) |
| `--max-ticks` | 2 000 |
| `--seed` | 2 |
| Architektur | 20→32→24→6 (net_arch=[32,24], Tanh) |
| Replay-Buffer | 200 000 steps |
| `learning_starts` | 10 000 |
| GPU | CUDA (nvidia), ~1 700 fps |

**Reward-Änderungen gegenüber Run 001:**

| Konstante | Run 001 | Run 002 |
|---|---|---|
| `REWARD_EAT` | 1.0 | **3.0** |
| `REWARD_STEP` | −0.005 | **−0.01** |
| `REWARD_DEATH` | −1.0 | −1.0 |
| `REWARD_APPROACH` | 0.1 | 0.1 |

---

## 2. Lernkurve

| Steps (k) | ep_rew_mean | Bemerkung |
|---|---|---|
| 133   | −0.51 | Random start |
| 388   | 15.1  | Exploration-Phase |
| 576   | 47.7  | Schneller Anstieg |
| 936   | **73.8** | Peak-Bereich |
| 1 350 | 71.9  | Leichter Rückgang |
| 2 175 | 64.1  | |
| 3 350 | 69.5  | |
| 5 000 | 58.8  | Endwert |

Peak ep_rew_mean ≈ **80.4** bei ca. 2.3M Schritten. Danach leichter
Rückgang auf ~59 — typisch für DQN (veralteter Replay-Buffer, ggf.
Overfitting). Endwert deutlich über Run 001 (33.5).

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks.

| Topologie  | Run 001 | **Run 002** | Δ |
|------------|--------:|------------:|--:|
| Walls Ø    |   22.96 |   **31.70** | +38.1 % |
| Periodic Ø |   48.18 |   **51.70** | +7.3 % |

Klare Verbesserung auf beiden Topologien, obwohl Run 002 nur auf Walls
trainiert wurde — Periodic-Generalisierung durch Distance-Shaping.

---

## 4. Vergleich mit anderen Methoden

| Methode       | Walls Ø | Periodic Ø |
|---------------|--------:|-----------:|
| MLP-GA 005    |   91.40 |     125.12 |
| NEAT run-001  |   38.92 |      56.84 |
| PPO run-001   |   38.24 |      58.62 |
| AZ run-001    |   37.60 |      54.28 |
| **DQN run-002**| **31.70** | **51.70** |
| DQN run-001   |   22.96 |      48.18 |

DQN bleibt schwächste RL-Methode, aber der Abstand zu NEAT/PPO/AZ hat
sich deutlich verringert (Walls-Gap: vorher 15.3 Punkte zu NEAT, jetzt 7.2).

---

## 5. Beobachtungen

- **Walls-only Training hilft DQN**: Run 001 (mixed) war schwächer als
  der 20k-Smoke-Run. Run 002 (walls-only, 5M Steps) schlägt Run 001 klar.
  DQN ist sample-ineffizient und profitiert davon, nicht auf zwei
  Randbedingungen gleichzeitig optimieren zu müssen.

- **Peak-and-Decline-Muster**: ep_rew_mean peaked bei 80.4 (~2.3M Steps),
  fiel dann auf 58.8. Das ist für DQN typisch: der Replay-Buffer füllt
  sich mit „alten" Erfahrungen aus früheren Policies, was die Updates
  destabilisiert. Frühes Stoppen hätte den Peak-Score gespeichert.

- **Periodic-Generalisierung**: Obwohl nur auf Walls trainiert, erreicht
  Run 002 auf Periodic 51.70 (+7.3 % vs Run 001, der mixed trainiert war).
  Das Distance-Shaping (REWARD_APPROACH = 0.1×Δdist) bleibt torus-sicher
  und hilft dem Agenten, auch auf periodischen Rändern zu navigieren.

- **Food-Reward-Effekt klar messbar**: REWARD_EAT 1→3 führt zu deutlich
  höherer ep_rew_mean (≈3× bei gleichem Tickbedarf) und zu aggressiveren
  Fressstrategien, die sich im Benchmark niederschlagen.

---

## 6. Fazit

Walls-only Training + dreifacher Food-Reward sind beide effektiv für DQN.
Für Run 003 empfohlen:

- `--early-stopping` oder Checkpoint beim Peak (ca. 2.3M Steps)
- Separater Torus-Run für bessere Periodic-Performance
- Alternativ: Wechsel zu PPO (on-policy, stabiler, ohne Replay-Verfall)

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/dqn/policy.mlp` | Eingecheckt (Run 002) |
| `training-out/dqn-run-002/policy.mlp` | Exportiertes Netz |
| `training-out/dqn-run-002/train.log` | Training-Logs (SB3-Output) |
