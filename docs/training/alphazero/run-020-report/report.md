# Training Report: AlphaZero-light — Run 020

**Datum**: 2026-06-15  
**Ziel**: Testen ob `--games-per-iter 256` (doppelte Datenmenge) balanced-Policies fördert.
Baseline: Run 016 Seed 15 (Walls 37.26, Periodic 64.96, Avg 51.11 — 200 Spiele).

---

## 1. Setup (alle 8 Seeds identisch außer `--seed`)

| Parameter | Wert |
|---|---|
| `--iterations` | 80 |
| `--boundary` | mixed |
| `--eat-bonus` | 0.3 |
| `--max-ticks` | 1 500 |
| `--lr` | 1e-3 |
| `--games-per-iter` | **256** (doppelt vs. vorher 128) |
| `--sims` | 24 |
| `--hidden` | 32 24 (Standard) |
| `--seed` | 40–47 |

Hypothese: Mehr Spiele pro Iteration → robustere Gradienten → balanced statt spezialisiert.

---

## 2. Benchmark-Ergebnis (50 Spiele)

| Seed | Walls best | P best | Avg best | Walls final | P final | Avg final |
|-----:|-----------:|-------:|---------:|------------:|--------:|----------:|
| Baseline (Run 016 s15) | 37.26 | 64.96 | 51.11 | — | — | — |
| s40 | 38.56 | 41.48 | 40.02 | 35.70 | 43.28 | 39.49 |
| s41 | 28.92 | 50.00 | 39.46 | 29.68 | 44.62 | 37.15 |
| s42 | 23.28 | 55.58 | 39.43 | 25.82 | 51.34 | 38.58 |
| s43 | 27.56 | 44.62 | 36.09 | 29.22 | 38.52 | 33.87 |
| s44 | 36.98 | 56.36 | 46.67 | 33.92 | 60.54 | 47.23 |
| s45 | 0.48 (!) | 49.76 | 25.12 | 37.26 | 49.96 | 43.61 |
| s46 | 23.16 | 36.24 | 29.70 | 21.34 | 37.90 | 29.62 |
| s47 | 32.66 | 57.66 | 45.16 | 34.14 | 60.10 | 47.12 |

**Kein Checkpoint übertrifft die Baseline.** Hypothese widerlegt: games-per-iter=256 hilft nicht.

Bestes Ergebnis: s44/final (Walls 33.92, Periodic 60.54, avg 47.23) — deutlich unter Baseline.

---

## 3. Analyse

### 3.1 games-per-iter=256 verschlechtert Training

Alle 8 Seeds liegen weit unter der Baseline. Mehrere Befunde:
- **Spezialisierung verstärkt sich**: s40 und s45 entwickeln extreme Walls-Stärke, aber null Periodic
- **Seed 45 best.mlp = Walls 0.48**: Snake kreist ewig (avg_ticks=8000!), frisst kaum
  → Der Peak-game_len Checkpoint entspricht einem schlechten Policy-Zustand
- **Seed 45 final.mlp**: Walls 37.26 — deutlich besser als best.mlp, aber Periodic nur 50

### 3.2 Warum hilft mehr Daten nicht?

Hypothese: Mit 128 Spielen/Iter lernt das Netz schnell, reagiert sofort auf neue Self-Play-Daten.
Mit 256 Spielen/Iter ist der Gradient über mehr (ältere + neue) Daten gemittelt — das führt zu
trägererer Anpassung. Der Sweet-Spot bei game_len 700–900 wird langsamer oder gar nicht erreicht.

Zudem: Der Rolling Buffer (60k Samples) läuft mit 256 Spielen/Iter schneller voll → ältere
Samples werden früher verdrängt. Das ändert die Trainings-Dynamik.

### 3.3 Schlussfolgerung

Standard `--games-per-iter 128` ist die optimale Konfiguration. Die Baseline (Run 016 Seed 15)
bleibt deployed.

**Nächster Schritt (Run 021 — letzter der 5 Läufe)**:
Frische Seeds 60–68 mit bewährter Standard-Konfiguration (games-per-iter=128, 80 iter).
Letzte Chance auf einen besseren balanced Seed.

---

## 4. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-020-s{40..47}/best.mlp` | Beste Checkpoints je Seed (alle unter Baseline) |
| `training-out/az-run-020-s{40..47}/final.mlp` | Finale Checkpoints je Seed |
