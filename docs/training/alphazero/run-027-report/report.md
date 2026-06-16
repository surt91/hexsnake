# Training Report: AlphaZero-light — Run 027 (No Hunger, 6 h)

**Datum**: 2026-06-16
**Ziel**: Nach dem Verwerfen des Hunger-Features (Run 026) die bewährte
Run-025-Konfiguration (20-Input, Board-Vielfalt, greedy-Eval-Auswahl) mit
**6 h Budget** statt ~30 min fahren — testet, ob deutlich längeres Training
einen besseren Checkpoint als Run 025 (iter 230) findet.

Referenz (deployed): Run 025 Seed 1 — Walls 48.21, Periodic 72.45, Avg 60.33.

---

## 1. Setup

| Parameter | Wert |
|---|---|
| `--max-hours` | 6 (→ **19 826 Iterationen**) |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--max-ticks` | 1500 |
| `--seed` | 1 |
| `--eval-every` / `--eval-games` / `--eval-max-ticks` | 5 / 20 / 4000 |
| Checkpoint-Auswahl | greedy Eval-Mittel (Walls+Periodic) |

## 2. Verlauf

Der beste Eval-Checkpoint liegt **sehr früh** (iter 815, ~15 min,
W 51.4 / P 87.1 / avg 69.2). Die restlichen ~5,75 h / 19 000 Iterationen finden
**kein höheres Eval-Mittel** — die Eval oszilliert danach zwischen Walls-starken
und Periodic-starken Zuständen. Großes Compute-Budget bestätigt also vor allem
das Plateau.

## 3. Benchmark (`bench_mlp`, 200 Spiele, 8000 Ticks, sims 24)

| Netz | Walls | Periodic | Avg |
|---|---:|---:|---:|
| Run 025 (deployed) | **48.21** | 72.45 | 60.33 |
| **Run 027 best (iter 815)** | 45.73 | **78.44** | **62.09** |
| Run 027 final (iter 19825) | **63.09** | 57.45 | 60.27 |

## 4. Analyse

- **Run 027 best schlägt Run 025 im Mittel** (62.09 vs 60.33, +2,9 %), getragen
  von Periodic (+8,3 %) bei leichtem Walls-Rückgang (−5,1 %). **Deployed.**
- **Faszinierend: der finale Checkpoint ist ein Walls-Spezialist** (Walls 63.09,
  +31 % — aber Periodic nur 57.45). Über die 6 h driftet die Policy zu extremer
  Walls-Stärke; das Eval-Mittel bleibt aber unter dem frühen Optimum, weil
  Periodic stark einbricht. Beleg dafür, dass die einfache `(W+P)/2`-Auswahl
  balancierte Checkpoints bevorzugt.
- **Langes Budget ≠ besser**: Wie Run 026 plateaut die greedy-Eval früh
  (hier ~15 min). Der Lernhebel ist nicht mehr „mehr Iterationen", sondern die
  Auswahl-/Reward-Struktur. Kein Reward-Kollaps (dank Run-024-Fixes).

## 5. Projektstand (deployed, 200 Spiele)

| Run | Walls | Periodic | Avg |
|-----|------:|---------:|----:|
| Run 021 s68 | 40.72 | 75.74 | 58.23 |
| Run 025 s1  | 48.21 | 72.45 | 60.33 |
| **Run 027 s1** | 45.73 | **78.44** | **62.09** |

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-027-s1/best.mlp` | **iter 815 (W 45.73 / P 78.44 / Avg 62.09) — deployed** |
| `training-out/az-run-027-s1/final.mlp` | iter 19825 (Walls-Spezialist, W 63.09 / P 57.45) |
| `crates/snake-core/assets/alphazero/best.mlp` | **Run 027 best — deployed** |
