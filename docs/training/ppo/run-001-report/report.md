# Training Report: PPO — Run 001

**Datum**: 2026-06-14  
**Ziel**: Erster produktiver PPO-Lauf mit gemischten Rändern.
Baseline bisher: Smoke-Run (Walls Ø 35, Periodic Ø 58), trainiert auf `--boundary walls`.

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
| GPU | CUDA (nvidia), ~1 500–1 900 fps |
| Reward | +1 Fressen, −1 Tod, ±0.1×Δdist, −0.005/Tick |

**Besonderheit**: PPO läuft mit 8 parallelen Environments (SubprocVecEnv). Mit
`max-ticks=2000` und den kurzen rollout-Buffern (n_steps=256 × 8 Envs = 2048)
sind die meisten Episoden noch nicht abgeschlossen wenn der Update-Schritt
startet — SB3 gibt daher kein `ep_rew_mean` in den Logs aus.

---

## 2. Lernkurve

PPO-Metriken aus den SB3-Logs (kein ep_rew_mean verfügbar — Episoden zu lang):

| Steps (k) | entropy_loss | explained_var | Bemerkung |
|---|---|---|---|
| 32      | −1.78 | −0.32 | Initiale, fast-zufällige Policy |
| 1 000   | −0.54 |  +0.36 | Policy-Entropie sinkt, Value-Fit steigt |
| 2 000   | −0.39 |  +0.70 | |
| 3 000   | −0.31 |  +0.87 | |
| 4 000   | −0.30 |  +0.92 | |
| 5 000   | −0.25 |  +0.96 | Konvergiert, Policy wird deterministischer |

`explained_variance` steigt von −0.32 auf 0.96 — der Value-Head passt sich gut
an die tatsächlichen Returns an. Entropy-Loss sinkt von −1.78 auf −0.25, Policy
wird über die 5M Steps stetig deterministischer.

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks.

| Topologie  | Smoke-Run (walls-only) | **Run 001** | Δ |
|------------|----------------------:|------------:|--:|
| Walls Ø    |                 34.70 |   **38.24** | +10.2 % |
| Periodic Ø |                 58.14 |   **58.62** | +0.8 % |

Klare Verbesserung auf Walls (+10 %) trotz gemischtem Training; Periodic
nahezu identisch.

---

## 4. Vergleich mit anderen Methoden

| Methode       | Walls Ø | Periodic Ø |
|---------------|--------:|-----------:|
| MLP-GA 005    |   91.40 |     125.12 |
| NEAT run-001  |   38.92 |      56.84 |
| **PPO run-001**| **38.24** | **58.62** |
| AZ run-001    |   37.90 |      54.14 |
| DQN run-001   |   22.96 |      48.18 |

PPO liegt auf einem Niveau mit NEAT und AlphaZero-light. Alle drei RL/EA-Methoden
deutlich unter MLP-GA (2.4× mehr Score). PPO ist 1.7× stärker als DQN.

---

## 5. Beobachtungen

- **PPO vs DQN**: PPO mit 5M Steps (On-Policy, 8 Envs) ist deutlich stärker als
  DQN mit 3M Steps (22.96 Walls / 48.18 Periodic). Das ist erwartet — PPO ist
  sample-effizienter durch On-Policy-Updates und Parallelisierung.

- **Mixed-Training funktioniert bei PPO**: Walls 38.24 > Smoke-Run 34.70 (auf
  Walls-only trainiert). PPO profitiert von der größeren Datenmenge durch
  gemischtes Training, ohne die Walls-Performance zu verlieren. DQN litt unter
  demselben Ansatz.

- **Plateau-Effekt**: entropy_loss und explained_variance konvergieren beide
  früh (~1M Steps) und verbessern sich danach kaum noch schnell. Das Budget von
  5M Steps ist ausreichend ausgenutzt; weitere Verbesserungen würden ein
  größeres Budget oder andere Hyperparameter erfordern.

- **GPU-Warnung**: SB3 warnt, dass MlpPolicy auf GPU schlechter als CPU sein kann.
  ~1500–1900 fps wurden trotzdem erreicht (vs. ~1700 fps DQN). PPO mit CPU und
  `--n-envs 8` wäre vermutlich schneller.

- **Keine ep_rew_mean-Logs**: Mit `max-ticks=2000` und n_steps=2048 (8 Envs ×
  256 Steps) enden die meisten Episoden nicht innerhalb eines Rollouts. SB3
  aggregiert `ep_rew_mean` nur für abgeschlossene Episoden.

---

## 6. Fazit

PPO ist die stärkste RL-Methode in diesem Vergleich (run-001) und erzielt
~38/59 auf Walls/Periodic — auf dem Niveau von NEAT und AlphaZero-light. Das
gemischte Training gelingt PPO gut.

**Für Run 002** empfohlen:
- `device='cpu'` explizit setzen (GPU-Warnung beheben)
- Mehr Budget: 10M Steps
- Boundary-separate Läufe: einmal nur Walls, einmal nur Torus
- `--n-steps 1024` + `--n-envs 16` für bessere Sample-Effizienz

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/ppo/policy.mlp` | Eingecheckt (Run 001) |
| `training-out/ppo-run-001/policy.mlp` | Exportiertes Netz |
| `training-out/ppo-run-001/train.log` | Training-Logs (SB3-Output) |
