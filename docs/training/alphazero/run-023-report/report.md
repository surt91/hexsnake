# Training Report: AlphaZero-light — Run 023

**Datum**: 2026-06-15
**Ziel**: Checkpoint-Auswahl nach einem *greedy* Benchmark statt nach
Self-Play-Score (Lehre aus Run 022). Ein Seed (1), 150 Iterationen.

Baseline (deployed): Run 021 Seed 68 — Walls 40.72, Periodic 75.74, Avg 58.23.

---

## 1. Fix in diesem Lauf

Neue Funktion `eval_net` in `train_alphazero.py`: spielt alle `--eval-every`
Iterationen `--eval-games` **greedy** (Temperatur 0) Spiele je Topologie und
mittelt den Score. `best.mlp` wird nach dem Walls+Periodic-Mittel gewählt.
Greedy-Spiel deckt Kreis-Kollaps auf (hohe Ticks, niedriger Score), was
stochastisches Self-Play verschleiert.

## 2. Ergebnis

Eval-Kurve sah stark aus: iter 145 eval W 64 / P 88 (avg 76). `best.mlp` = iter
145. Aber `bench_mlp` (greedy, 8000 Ticks, Seeds 0..49):

| Netz | Walls | Periodic | avg_ticks (Walls) |
|---|---:|---:|---:|
| Champion s68 | 40.04 | 67.40 | 1098 |
| Run 023 best (iter 145) | **18.16** | 49.66 | **4869 (kreist!)** |

## 3. Analyse / Lehre (der eigentliche Bug)

Eval und Benchmark widersprachen sich krass (Eval W 64 vs. Bench W 18) — für
*dasselbe* Netz. Ursache, beim Debuggen gefunden:

- `az_selfplay` setzte den **Board-Seed hart auf 0**. Nur der Aktions-Sampling-
  RNG variierte. Bei Temperatur 0 (greedy Eval) ist der aber ungenutzt → **alle
  12 Eval-Spiele je Topologie waren dasselbe deterministische Spiel** auf Board
  0. Die Eval war also ein einzelnes (zufällig „leichtes") Board, kein Mittel.
- Schlimmer: Auch das **Training** lief immer auf Board-Seed 0. Die Policy sah
  nie die Board-Vielfalt (Seeds 0..N), auf der der Benchmark misst → sie
  überfittete Board 0 und fuhr auf unbekannten Boards im Kreis.

→ **Nächster Schritt (Run 024)**: Board-Seed pro Spiel variieren (Training:
diverse Boards; Eval: Board-Seeds 0..N wie `bench_mlp`).
