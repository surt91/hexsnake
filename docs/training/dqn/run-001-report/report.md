# Training Report: DQN — Run 001

**Datum**: 2026-06-14  
**Ziel**: Erster produktiver DQN-Lauf mit gemischten Rändern.
Baseline bisher: Smoke-Run (Walls Ø 28, Periodic Ø 48), trainiert auf `--boundary walls`.

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
| Replay-Buffer | 200 000 steps |
| `learning_starts` | 10 000 |
| GPU | CUDA (nvidia), ~1 700 fps |
| Reward | +1 Fressen, −1 Tod, ±0.1×Δdist, −0.005/Tick |

**Hinweis zum Smoke-Run**: Die Baseline (`--timesteps 20000`, `--boundary walls`)
wurde auf Walls-only trainiert. Run 001 trainiert gemischt (50/50) mit 150× mehr
Budget — der direkte Vergleich ist nicht apples-to-apples.

---

## 2. Lernkurve

| Steps (k) | ep_rew_mean | Bemerkung |
|---|---|---|
| 34      | 0.01 | Warming up, noch random |
| 500     | 31.5 | Schnell konvergiert |
| 1 000   | 31.0 | Stagnation |
| 1 500   | 29.5 | Leichter Rückgang |
| 2 000   | 36.7 | Erholung |
| 2 500   | 34.4 | Keine klare Verbesserung |
| 3 000   | 33.5 | Endwert, kaum Progress seit 500k |

ep_rew_mean schnell auf ~31 konvergiert und danach kaum verbessert — typisch
für DQN: Sample-ineffizient, braucht viel mehr Budget als 3M Steps für diesen
Schwierigkeitsgrad.

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks.

| Topologie  | Smoke-Run (walls-only) | **Run 001** | Δ |
|------------|----------------------:|------------:|--:|
| Walls Ø    |                 28.18 |   **22.96** | −18.5 % |
| Periodic Ø |                 48.08 |   **48.18** | +0.2 % |

Walls-Score schlechter als der Smoke-Run (mit nur 20k Steps auf Walls). Periodic
auf gleichem Niveau.

**Interpretation**: Das gemischte Training verteilt das Budget auf beide
Randbedingungen. Der DQN lernt keine Spezialisierung für Walls und verliert
Performance gegenüber dem Walls-only Smoke-Run. 3M Steps reichen für DQN nicht
aus, beide Randbedingungen gleichzeitig zu meistern.

---

## 4. Vergleich mit anderen Methoden

| Methode       | Walls Ø | Periodic Ø |
|---------------|--------:|-----------:|
| MLP-GA 005    |   91.40 |     125.12 |
| NEAT run-001  |   38.92 |      56.84 |
| PPO run-001   |   38.24 |      58.62 |
| AZ run-001    |   37.90 |      54.14 |
| **DQN run-001**| **22.96** | **48.18** |

DQN ist die schwächste Methode in diesem Vergleich. DQN vs. PPO zeigt den
erwarteten Unterschied: PPO (On-Policy, vektorisiert) mit 5M Steps ist deutlich
stärker als DQN (Off-Policy, single-env) mit 3M Steps.

---

## 5. Beobachtungen

- **Frühe Konvergenz, frühe Stagnation**: ep_rew_mean erreicht ~31 in den ersten
  500k Steps und verbessert sich danach kaum. DQN ist für diese Umgebung
  sample-ineffizient — der Reward-Signal (shaping über Distanz zum Essen) hilft
  dem Agenten zwar schnell Orientierung zu finden, reicht aber nicht für
  komplexere Strategien.

- **Mixed-Training schadet Walls**: Der Smoke-Run (Walls-only, 20k Steps) schlägt
  Run 001 (Mixed, 3M Steps) auf Walls (28.18 vs 22.96). Das gemischte Training
  erzeugt einen Generalisten, der keine Randbedingung wirklich meistert.

- **ep_len_mean 198 bei 3M Steps**: Kurze Episoden (198 Ticks im Schnitt) bedeuten,
  der Agent überlebt im Median weniger als 200 Ticks. Verglichen mit PPO
  (weniger direkt messbar, aber höherer Score) deutet das auf schlechtere
  Überlebensstrategien hin.

- **GPU-Nutzung ineffizient**: SB3 warnt explizit, dass DQN/MlpPolicy auf GPU
  meist langsamer ist als CPU (das Netz ist zu klein für effiziente
  Batch-Verarbeitung). ~1700 fps wurden erreicht.

---

## 6. Fazit

DQN mit 3M Steps auf gemischten Rändern liefert schlechtere Ergebnisse als der
20k-Step Smoke-Run auf Walls. Für produktive DQN-Ergebnisse empfehlen sich:

- Separate Läufe für `--boundary walls` und `--boundary torus`
- Deutlich mehr Budget: ≥ 10M Steps
- Oder: Wechsel zu PPO, das bei 5M Steps deutlich besser abschneidet

Die Methode ist konzeptuell korrekt implementiert (Export-Roundtrip, identische
Architektur), aber für diese Aufgabe sample-ineffizient.

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/dqn/policy.mlp` | Eingecheckt (Run 001) |
| `training-out/dqn-run-001/policy.mlp` | Exportiertes Netz |
| `training-out/dqn-run-001/train.log` | Training-Logs (SB3-Output) |
