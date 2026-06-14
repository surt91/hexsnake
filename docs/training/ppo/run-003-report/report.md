# Training Report: PPO — Run 003

**Datum**: 2026-06-14  
**Ziel**: Mixed-Boundary-Training mit originalen Reward-Konstanten (kein ×3-Eat)
und doppeltem Budget (10M vs. 5M Schritte). Run 002 hatte REWARD_EAT×3, was auf
mixed boundary zu einer Regression führte (−9.9 % Walls).
Baseline: Run 001 (Walls Ø 38.24, Periodic Ø 58.62).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | PPO (stable-baselines3) |
| `--timesteps` | 10 000 000 |
| `--n-envs` | 8 |
| `--boundary` | **mixed** (50/50 Walls/Torus) |
| `--max-ticks` | 2 000 |
| `--seed` | 3 |
| Architektur | 20→32→24→6 (pi+vf net_arch=[32,24], Tanh) |
| GPU | CUDA (nvidia) |

**Reward-Konstanten** (identisch mit Run 001):

| Konstante | Wert |
|---|---|
| `REWARD_EAT` | 1.0 |
| `REWARD_STEP` | −0.005 |
| `REWARD_DEATH` | −1.0 |
| `REWARD_APPROACH` | 0.1 |

---

## 2. Lernkurve

SB3-Metriken (kein `ep_rew_mean` — Episoden zu lang für SB3-Rollout-Buffer):

| Steps | explained_var | entropy_loss | Bemerkung |
|---|---|---|---|
| 0.0M | −0.150 | −1.780 | Initiale zufällige Policy |
| 0.9M |  0.922 | −0.534 | Schnelles Lernen |
| 1.8M |  0.947 | −0.358 | |
| 2.7M |  0.945 | −0.248 | Konvergenz |
| 3.6M |  0.948 | −0.239 | |
| 4.5M |  0.958 | −0.212 | |
| 5.4M |  0.958 | −0.221 | |
| 6.3M |  0.949 | −0.213 | |
| 7.2M |  0.949 | −0.197 | |
| 8.1M |  0.956 | −0.209 | |
| 9.0M |  0.943 | −0.234 | |
| 10.0M | 0.957 | −0.198 | Endwert |

**Beobachtung**: explained_variance stabilisiert sich ab ~1M Schritten bei ~0.95
und bleibt stabil bis 10M. Entropy sinkt gleichmäßig von −1.78 auf −0.20 —
die Policy wird deterministischer aber nicht kollabiert.

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks.

| Topologie  | Run 001 | Run 002 | **Run 003** | Δ vs. 001 |
|------------|--------:|--------:|------------:|----------:|
| Walls Ø    |   38.24 |  ~28.34 |   **42.16** | **+10.2 %** |
| Periodic Ø |   58.62 |  ~49.56 |   **58.28** |    −0.6 % |

**Run 003 ist das neue Beste** — übertrifft Run 001 auf Walls um +10.2 %;
Periodic ist innerhalb der Messungenauigkeit (−0.6 %).

---

## 4. Analyse

### 4.1 Doppeltes Budget hilft auf Walls

Run 001 (5M Schritte, mixed) erreichte 38.24 Walls. Run 003 (10M, gleiche
Reward-Funktion) erreicht 42.16 (+10.2 %). Die Policy konvergiert nicht nach
5M — mehr Budget verbessert Walls-Taktik.

### 4.2 Periodic bleibt stabil

58.28 vs. 58.62 (−0.6 %) ist innerhalb der Stichproben-Schwankung (50 Spiele).
Mixed-Training hilft Periodic im Vergleich zu walls-only, ohne es zu schaden.

### 4.3 REWARD_EAT×3 war das Problem in Run 002

Run 002 (×3-Eat, mixed) hatte Walls ~28.34. Run 003 (×1, mixed) hat 42.16.
Auf Walls sind aggressivere Food-Strategien (×3) kontraproduktiv — die Schlange
riskiert zu viel in Wandnähe. Die original +1.0 Reward-Skala ist besser für
mixed-boundary PPO.

---

## 5. Schlussfolgerungen

**Run 003 deployed** als neues `crates/snake-core/assets/ppo/policy.mlp`.

Mögliche nächste Schritte:
- **15–20M Schritte**: Die Policy konvergiert noch — mehr Budget könnte
  weiter verbessern.
- **n_steps tunen**: Größere Rollout-Buffer für korrekte `ep_rew_mean`-Logs.
- **LR-Schedule**: LinearSchedule (0.0003 → 0) für stabilere End-Phase.

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/ppo-run-003/policy.mlp` | Gewichte Run 003 |
| `training-out/ppo-run-003/train.log` | SB3-Log (10M Schritte) |
| `crates/snake-core/assets/ppo/policy.mlp` | **Run 003 embedded** |
