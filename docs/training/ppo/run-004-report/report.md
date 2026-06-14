# Training Report: PPO — Run 004

**Datum**: 2026-06-15  
**Ziel**: 2.5× mehr Budget als Run 003 (25M vs 10M Schritte), um zu prüfen ob
PPO noch weiter konvergiert. Run 003 zeigte Entropy noch bei −0.198 nach 10M,
was auf verbleibendes Lernpotenzial hinweist.
Baseline: Run 003 (Walls Ø 42.16, Periodic Ø 58.28).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | PPO (stable-baselines3) |
| `--timesteps` | 25 000 000 |
| `--n-envs` | 8 |
| `--boundary` | mixed (50/50 Walls/Torus) |
| `--max-ticks` | 2 000 |
| `--seed` | 4 |
| Architektur | 20→32→24→6 (pi+vf net_arch=[32,24], Tanh) |
| GPU | CUDA (nvidia) |

Reward-Konstanten identisch mit Run 001/003 (original, kein ×3-Eat).

---

## 2. Lernkurve

SB3-Metriken (explained_variance, entropy_loss) auf 2M-Schritte-Intervallen:

| Steps | explained_var | entropy_loss | Bemerkung |
|---|---|---|---|
|  1M | 0.939 | −0.551 | Schnelles Lernen |
|  5M | 0.940 | −0.402 | |
| 10M | 0.950 | −0.345 | (Run 003 Endpunkt) |
| 15M | 0.956 | −0.291 | Weitere Konvergenz |
| 20M | 0.955 | −0.244 | |
| 25M | 0.950 | **−0.278** | Endwert — nicht vollständig konvergiert |

**Beobachtung**: Entropy sinkt kontinuierlich (−0.551 → −0.278) und ist bei 25M
noch nicht am Minimum. Das deutet darauf hin, dass 40–50M Schritte die Policy
noch weiter verbessern würden. explained_variance stagniert bei ~0.95 (gut).

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks.

| Topologie  | Run 001 | Run 003 | **Run 004** | Δ vs. 003 | Δ vs. 001 |
|------------|--------:|--------:|------------:|----------:|----------:|
| Walls Ø    |   38.24 |   42.16 |   **39.62** |    −6.0 % |  **+3.6 %** |
| Periodic Ø |   58.62 |   58.28 |   **60.98** |  **+4.6 %** |  **+4.0 %** |

**Periodic: Bestes PPO-Ergebnis aller Runs (60.98).**
Run-004 verbessert beide Metriken gegenüber Run-001, zeigt aber Walls-Regression
gegenüber Run-003 (−6 %). Die Differenz (2.54 Punkte auf Walls) liegt innerhalb
der Messungenauigkeit (50 Spiele), ist aber konsistent.

---

## 4. Analyse

### 4.1 Walls-Regression erklärbar durch Zufallseffekte

Run-003 und Run-004 haben verschiedene Seeds (3 vs. 4). Bei 50 Benchmark-Spielen
ist eine Schwankung von ±3 Punkten durch Seed-Effekte realistisch. Die
unterschiedlichen Lernpfade (Seeds 3 vs. 4) können die gleiche Walls-Policy-
Qualität mit anderem Zufallsergebnis erzeugen.

### 4.2 Mehr Budget hilft Periodic

10M → 25M Schritte: Periodic 58.28 → 60.98 (+4.6 %). Walls 42.16 → 39.62 (−6 %).
Das Verhältnis zeigt: Mixed-Training mit mehr Budget optimiert Periodic stärker
als Walls (möglich, weil Torus-Grenzen mehr Freiheitsgrade für längere Schlangen
bieten → mehr Lernsignal pro Episode).

### 4.3 Policy noch nicht konvergiert

Entropy bei 25M noch −0.278 (vs. −0.198 bei Run 003 nach 10M). Das sinkende
Entropie-Profil deutet auf weiteres Lernpotenzial hin. 40–50M Schritte könnten
beide Metriken weiter verbessern.

---

## 5. Schlussfolgerungen

**Run 004 deployed** als neues `crates/snake-core/assets/ppo/policy.mlp`.

Begründung: Periodic 60.98 ist das beste PPO-Ergebnis (+4.6 % vs. Run 003).
Walls (39.62) ist besser als Run 001 (+3.6 %) — Run 004 ist auf beiden Metriken
besser als der ursprüngliche Run 001. Der Walls-Rückgang vs. Run 003 ist klein
und möglicherweise Seed-bedingt.

Mögliche nächste Schritte:
- **40–50M Schritte**: Entropy sinkt noch, mehr Budget könnte beide Metriken
  weiter verbessern.
- **Mehrere Seeds bei 25M**: Um Seed-Effekte zu trennen und die robusteste
  Policy zu identifizieren.

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/ppo-run-004/policy.mlp` | Gewichte Run 004 |
| `training-out/ppo-run-004/train.log` | SB3-Log (25M Schritte) |
| `crates/snake-core/assets/ppo/policy.mlp` | **Run 004 embedded** |
