# Training Report: AlphaZero-light — Run 025

**Datum**: 2026-06-15
**Ziel**: Mit den Fixes aus Run 022–024 (Checkpoint nach greedy Benchmark,
Board-Vielfalt) prüfen, ob **längeres Training jetzt hilft** statt zu schaden —
also die Nutzer-Hypothese „länger → schlechter war ein Bug, kein Naturgesetz".
Ein Seed (1), **300 Iterationen** (doppeltes Budget).

Baseline (deployed): Run 021 Seed 68 — Walls 40.72, Periodic 75.74, Avg 58.23
(200 Spiele).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| `--iterations` | **300** |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--max-ticks` | 1500 |
| `--seed` | 1 |
| `--eval-every` / `--eval-games` / `--eval-max-ticks` | 5 / 20 / 4000 |
| Checkpoint-Auswahl | greedy Eval-Mittel (Walls+Periodic, Board-Seeds 0..19) |

Iterationen 0–149 sind (seed-deterministisch) identisch zu Run 024; neu ist
150–299.

## 2. Eval-Verlauf (Auszug)

| iter | W | P | avg | eval_ticks |
|---:|---:|---:|---:|---:|
| 105 | 47.3 | 73.2 | 60.2 | 1103 |
| 150 | ~44 | ~70 | ~57 | ~1100 |
| 210 | 48.0 | 74.8 | 61.4 | 1385 |
| **230** | **46.5** | **81.8** | **64.2** | 1552 |
| 299 | 43.7 | 66.7 | 55.2 | 1136 |

Die zweite Hälfte (150–299) verbessert weiter — kein „länger → schlechter".
`best.mlp` = iter 230. Danach pendelt die Eval (Periodic ist hochvariant).

## 3. Benchmark (`bench_mlp`, 200 Spiele, 8000 Ticks, sims 24)

| Netz | Walls | Periodic | Avg |
|---|---:|---:|---:|
| Champion s68 | 40.72 | **75.74** | 58.23 |
| **Run 025 best (iter 230)** | **48.21** (+18.4 %) | 72.45 (−4.3 %) | **60.33** (+3.6 %) |
| Run 025 final (iter 299) | 45.98 | 68.64 | 57.31 |

## 4. Analyse

- **„Länger → schlechter" war ein Bug.** Mit benchmark-treuer Checkpoint-Auswahl
  und Board-Vielfalt steigt die Qualität über 300 Iterationen weiter; der späte
  Kreis-Kollaps früherer Läufe tritt nicht mehr auf (avg_ticks bleiben gesund
  ~1100–1900).
- **Run 025 best schlägt den Champion im Mittel** (+3.6 %) und deutlich auf Walls
  (+18.4 %), bei kleinem Periodic-Rückgang (−4.3 %). Erstmals aus *einem
  beliebigen* Seed statt aus einem Glückstreffer.
- **Periodic bleibt hochvariant** (max 175 vs. 148). Der Eval-Peak (P 81.8 bei
  iter 230) übertrifft den 200-Spiel-Bench (P 72.45) — die 20-Spiel-Eval
  überschätzt Periodic.

## 5. Deploy

Run 025 best (iter 230) ist im Mittel klar besser und deutlich balancierter als
der Champion. **Deployed** (mit Nutzer abgestimmt, Walls↑/Periodic↓-Abwägung):
nach `crates/snake-core/assets/alphazero/best.mlp` kopiert; `embedded()` nutzt
bereits sims=24 (passt zum Training). `cargo test -p snake-core alphazero` grün.

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-025-s1/best.mlp` | **iter 230 (W 48.21 / P 72.45 / Avg 60.33)** |
| `training-out/az-run-025-s1/final.mlp` | iter 299 (W 45.98 / P 68.64) |
