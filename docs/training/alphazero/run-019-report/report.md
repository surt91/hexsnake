# Training Report: AlphaZero-light — Run 019

**Datum**: 2026-06-15  
**Ziel**: Breiterer Seed-Sweep (Seeds 30–38) um die Baseline zu übertreffen.
Baseline: Run 016 Seed 15 (Walls 37.26, Periodic 64.96, Avg 51.11 — 200 Spiele).

---

## 1. Setup (alle 9 Seeds identisch außer `--seed`)

| Parameter | Wert |
|---|---|
| `--iterations` | 80 |
| `--boundary` | mixed |
| `--eat-bonus` | 0.3 |
| `--max-ticks` | 1 500 |
| `--lr` | 1e-3 |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--hidden` | 32 24 (Standard) |
| `--seed` | 30–38 |

---

## 2. Benchmark-Ergebnis (50 Spiele, beide Checkpoints)

| Seed | Walls best | P best | Avg best | Walls final | P final | Avg final |
|-----:|-----------:|-------:|---------:|------------:|--------:|----------:|
| Baseline (Run 016 s15) | 37.26 | 64.96 | 51.11 | — | — | — |
| s30 | 27.74 | 59.62 | 43.68 | 29.76 | 62.20 | 45.98 |
| s31 | 31.30 | 50.12 | 40.71 | 31.30 | 50.12 | 40.71 |
| s32 | 29.02 | **69.68** | 49.35 | 28.22 | 66.88 | 47.55 |
| s33 | 29.72 | 53.58 | 41.65 | 29.28 | 55.78 | 42.53 |
| s34 | 30.60 | 56.40 | 43.50 | 30.60 | 56.40 | 43.50 |
| s35 | 36.88 | 51.34 | 44.11 | 36.88 | 51.34 | 44.11 |
| s36 | 26.90 | 55.58 | 41.24 | 24.16 | 55.18 | 39.67 |
| **s37** | 34.20 | **72.82** | **53.51** | 34.20 | 72.82 | 53.51 |
| s38 | 27.00 | 55.84 | 41.42 | 23.00 | 55.08 | 39.04 |

### Seed 37 Verifikation (100 Spiele)

| Topologie | Walls | Periodic | Avg |
|-----------|------:|--------:|----:|
| Run 016 s15 (Baseline, 200 Sp.) | 37.26 | 64.96 | 51.11 |
| **Seed 37 best (100 Sp.)** | 34.83 | **70.85** | **52.84** |

**Seed 37 gewinnt auf kombinierter Metrik (+3.4%), verliert auf Walls (−6.5%).**
avg_ticks Walls: 3 584 (Seed 37) vs. 693 (Baseline) — Snake überlebt lange aber frisst
sehr ineffizient (102 Ticks/Apfel vs. 18.6 beim Baseline). Kein Deploy wegen starker
Walls-Regression.

---

## 3. Analyse

### 3.1 Erneute Spezialisierungsmuster

Run 019 zeigt wieder: Seeds spezialisieren sich. Seed 37 ist extremer Periodic-Spezialist
(Periodic 72.82, Walls 34.20). Seed 35 ist Walls-Spezialist (36.88, 51.34).
Kein Seed dominiert beide Metriken gleichzeitig.

### 3.2 Seed 37 — wertvoller Befund trotz kein Deploy

Periodic 72.82 ist der höchste je gemessene Wert für dieses Projekt. Wenn Periodic
als alleinige Metrik zählt, wäre Seed 37 die beste Policy. Aber Walls-Performance ist
zu schwach für ein balanced Deploy.

### 3.3 Nächster Schritt (Run 020)

Hypothese: `--games-per-iter 256` (doppelte Datenmenge pro Iteration) führt zu
robusteren Gradienten und balanced-Policies. 8 Seeds (40–47) mit neuem Parameter.

---

## 4. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-019-s37/best.mlp` | Seed 37 (Walls 34.83, Periodic 70.85) — Periodic-Rekord, nicht deployed |
| `training-out/az-run-019-s{30..38}/best.mlp` | Beste Checkpoints je Seed |
