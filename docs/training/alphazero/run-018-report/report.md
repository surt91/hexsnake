# Training Report: AlphaZero-light — Run 018

**Datum**: 2026-06-15  
**Ziel**: Verbesserung über Run 016 Seed 15 (Walls 37.26, Periodic 64.96).
Zwei parallele Ansätze: Warm-Start (Refinement) und neuer Seed-Sweep.

---

## 1. Setup

### 018A: Warm-Start (Refinement)

| Parameter | Wert |
|---|---|
| `--load` | `training-out/az-run-016-s15/best.mlp` |
| `--iterations` | 100 |
| `--lr` | **3e-4** (reduziert von 1e-3) |
| `--boundary` | mixed |
| `--eat-bonus` | 0.3 |
| `--max-ticks` | 1 500 |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--seed` | 1 |

### 018B: Neuer Seed-Sweep

| Parameter | Wert |
|---|---|
| `--iterations` | 80 |
| `--boundary` | mixed |
| `--eat-bonus` | 0.3 |
| `--max-ticks` | 1 500 |
| `--lr` | 1e-3 |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--seed` | 20 / 21 / 22 |

---

## 2. Benchmark-Ergebnis (50 Spiele)

| Config | ckpt | Walls Ø | Periodic Ø | Avg |
|--------|------|--------:|-----------:|----:|
| Baseline (Run 016 s15, 200 Sp.) | — | 37.26 | 64.96 | 51.11 |
| warmstart | best | 36.72 | 63.86 | 50.29 |
| warmstart | final | 34.30 | **65.30** | 49.80 |
| s20 | best | **41.06** | 50.24 | 45.65 |
| s20 | final | 39.16 | 45.84 | 42.50 |
| s21 | best | 30.38 | 52.06 | 41.22 |
| s21 | final | 31.00 | 60.08 | 45.54 |
| s22 | best | 36.26 | 57.92 | 47.09 |
| s22 | final | 35.70 | 57.08 | 46.39 |

**Kein Checkpoint übertrifft die Baseline.** Run 016 Seed 15 bleibt deployed.

---

## 3. Analyse

### 3.1 Warm-Start

Warm-Start von Seed 15 best.mlp (LR=3e-4, 100 Iter) erhält Periodic ~64-65,
aber Walls sinkt auf 34-37. Kein Gesamtgewinn (avg 49.8 vs. 51.11 Baseline).

### 3.2 Neue Seeds

- **Seed 20**: Walls-Spezialist (41.06!) aber schwaches Periodic (50.24) — bestätigt das Spezialisierungsmuster erneut
- **Seed 21 final**: Periodic 60, aber Walls nur 31 — unter der Baseline
- **Seed 22**: Durchschnittlich auf beiden Metriken

### 3.3 Schlussfolgerung

Baseline robust. Run 019: Breiterer Sweep (Seeds 30–38) — mehr Seeds = mehr
Chancen auf ein besseres Local Optimum.

---

## 4. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-018-warmstart/best.mlp` | Warm-Start (Walls 36.72, Periodic 63.86) |
| `training-out/az-run-018-s20/best.mlp` | Seed 20 (Walls 41.06, Periodic 50.24) |
| `training-out/az-run-018-s21/final.mlp` | Seed 21 (Walls 31.00, Periodic 60.08) |
| `training-out/az-run-018-s22/best.mlp` | Seed 22 (Walls 36.26, Periodic 57.92) |
