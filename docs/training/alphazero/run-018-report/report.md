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

## 2. Ergebnisse

*Werden nach Training-Abschluss ergänzt.*

---

## 3. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-018-warmstart/best.mlp` | 018A Warm-Start Best-Checkpoint |
| `training-out/az-run-018-s20/best.mlp` | 018B Seed 20 |
| `training-out/az-run-018-s21/best.mlp` | 018B Seed 21 |
| `training-out/az-run-018-s22/best.mlp` | 018B Seed 22 |
