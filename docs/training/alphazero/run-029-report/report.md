# Training Report: AlphaZero-light — Run 029 (Topologie-Bit + größeres Netz)

**Datum**: 2026-06-16
**Ziel**: Das Topologie-Bit (Run 028) mit **mehr Netz-Kapazität** kombinieren.
Hypothese: Run 028 zeigte, dass das Standardnetz die Topologie-Information zur
*Spezialisierung* nutzt statt zur Beherrschung beider Topologien — vielleicht
fehlt schlicht Kapazität für zwei Policies. Test: 21→**64→48**→7 (~4 900 Params,
vs. 1 671 Standard), kleinere LR (5e-4).

Referenz (deployed): Run 027 Seed 1 — Walls 45.73, Periodic 78.44, Avg 62.09.

---

## 1. Setup

| Parameter | Wert |
|---|---|
| `--hidden` | **64 48** (Netz 21→64→48→7) |
| `--lr` | **5e-4** (halbe Standard-LR; größeres Netz mag sanftere Schritte) |
| `--max-hours` | 1 (→ 2 404 Iterationen) |
| `--seed` | 1 |
| `--games-per-iter` / `--sims` / `--max-ticks` | 128 / 24 / 1500 |
| `--eval-every` / `--eval-games` / `--eval-max-ticks` | 5 / 20 / 4000 |

## 2. Verlauf

Bester Eval-Checkpoint iter 1640: **W 63.2 / P 76.0 / avg 69.6** — erstmals beide
Achsen gleichzeitig hoch (Run 027 best war Periodic-lastig W51/P87, Run 028
Walls-lastig W52/P81). Das größere Netz lernt etwas langsamer (lr 5e-4), erreicht
das Optimum nach ~40 min.

## 3. Benchmark (`bench_mlp`, 200 Spiele, 8000 Ticks, sims 24)

| Netz | Walls | Periodic | Avg |
|---|---:|---:|---:|
| Run 027 (deployed) | 45.73 | **78.44** | 62.09 |
| **Run 029 best (iter 1640)** | **53.03** (+16 %) | 75.45 (−3.8 %) | **64.24** (+3.5 %) |
| Run 029 final (iter 2403) | 46.37 | 70.20 | 58.29 |

## 4. Analyse — Kapazität + Konditionierung wirken zusammen

- **Bestes Netz bisher, und das ausgewogenste**: Walls 53.03 **und** Periodic
  75.45. Walls +16 % gegenüber Run 027, Periodic nur leicht (−3.8 %), Avg +3.5 %.
  **Deployed.**
- **Die Lehre der drei Läufe**: Topologie-Bit *allein* (Run 028) verschob nur die
  Balance (Walls↑/Periodic↓). Erst **Bit + Kapazität** (Run 029) ließ ein Netz
  beide Topologien gut spielen. Information ohne Kapazität reicht nicht — und
  Kapazität ohne die Topologie-Information hatte in Run 017 (ohne Board-Vielfalt)
  nichts gebracht. Es brauchte beide Fixes *davor* (Board-Vielfalt, greedy-Eval)
  plus beide Hebel zusammen.
- **Run 017 widerlegt im neuen Kontext**: Größere Netze galten als nutzlos
  (Run 017: 20→128→96→7, Walls 32/Periodic 44). Das lag an Board-Seed-0-Overfit
  und 150 Iterationen — mit Board-Vielfalt + Topologie-Bit hilft mehr Kapazität.

## 5. Projektstand (deployed, 200 Spiele)

| Run | Architektur | Walls | Periodic | Avg |
|-----|-------------|------:|---------:|----:|
| Run 021 s68 | 20→32→24→7 | 40.72 | 75.74 | 58.23 |
| Run 025 s1  | 20→32→24→7 | 48.21 | 72.45 | 60.33 |
| Run 027 s1  | 20→32→24→7 | 45.73 | 78.44 | 62.09 |
| **Run 029 s1** | **21→64→48→7 (+Topologie)** | **53.03** | 75.45 | **64.24** |

Gegenüber dem Sessionsstart (Run 021): Walls +30 %, Avg +10 %.

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-029-s1/best.mlp` | **iter 1640, 21→64→48→7 (W 53.03 / P 75.45 / Avg 64.24) — deployed** |
| `training-out/az-run-029-s1/final.mlp` | iter 2403 |
| `crates/snake-core/assets/alphazero/best.mlp` | **Run 029 best — deployed** |
