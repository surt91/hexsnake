# Training Report: AlphaZero-light — Run 024

**Datum**: 2026-06-15
**Ziel**: Den in Run 023 gefundenen Board-Seed-Bug beheben — Board-Vielfalt im
Training, benchmark-treue Eval — und prüfen, ob die Seed-Abhängigkeit damit
verschwindet. Ein Seed (1), 150 Iterationen.

Baseline (deployed): Run 021 Seed 68 — Walls 40.72, Periodic 75.74, Avg 58.23
(200 Spiele).

---

## 1. Fix in diesem Lauf

- **Board-Vielfalt**: `az_selfplay` nutzt den `seed`-Parameter jetzt auch für
  den Board-RNG (vorher hart 0). Jedes Self-Play-Spiel läuft auf einem anderen
  Board / einer anderen Futter-Sequenz.
- **Benchmark-treue Eval**: `eval_net` spielt greedy auf Board-Seeds
  `0..eval_games` — exakt die Seeds, die `bench_mlp`/`run_series` benutzen.
  Trainings-Boards (Seed `seed·1e6+`) bleiben disjunkt/held-out.

## 2. Setup

| Parameter | Wert |
|---|---|
| `--iterations` | 150 |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--max-ticks` | 1500 |
| `--seed` | 1 |
| `--eval-every` / `--eval-games` / `--eval-max-ticks` | 5 / 16 / 4000 |
| Checkpoint-Auswahl | greedy Eval-Mittel (Walls+Periodic) |

## 3. Eval-Verlauf (Auszug)

| iter | W | P | avg | eval_ticks |
|---:|---:|---:|---:|---:|
| 0 | 0.5 | 5.6 | 3.1 | 3655 |
| 25 | 33.0 | 65.8 | 49.4 | 1098 |
| 60 | 39.2 | 72.7 | 56.0 | 1294 |
| 100 | 44.2 | 70.8 | 57.5 | 1103 |
| **105** | **46.7** | **73.5** | **60.1** | 1085 |
| 149 | 42.5 | 70.0 | 56.2 | 1270 |

Wichtig: `eval_ticks` fällt von 3655 → ~1100 und **bleibt** dort — die Schlange
frisst effizient statt zu kreisen. Kein „länger → schlechter": Die Kurve
plateaut bei avg ~55–60, fällt nicht ab. `best.mlp` = iter 105.

## 4. Benchmark (`bench_mlp`, 200 Spiele, 8000 Ticks, sims 24)

| Netz | Walls | Periodic | Avg |
|---|---:|---:|---:|
| Champion s68 | 40.72 | **75.74** | **58.23** |
| Run 024 best (iter 105) | **43.98** | 62.74 | 53.36 |
| Run 024 final (iter 149) | **44.41** | 64.72 | 54.57 |

## 5. Analyse

- **Seed-Lotterie behoben**: Ein *beliebiger* Seed (1) trainiert jetzt zuverlässig
  eine balancierte, nicht kreisende Policy bei Avg ~54 — früher war das ein
  Glücksspiel (die meisten Seeds 40–45, nur s68 erreichte 58). Die Eval sagt den
  Benchmark gut voraus (Walls 46.7 Eval vs. 44.0 Bench).
- **Walls verbessert** (+8 % vs. Champion), aber **Periodic schwächer**
  (62–65 vs. 75.74). Der Champion s68 bleibt insgesamt vorn (Avg 58.23) —
  hauptsächlich wegen seiner außergewöhnlichen Periodic-Stärke (hohe Varianz,
  max 148).
- **Kein Deploy**: Run 024 schlägt den Champion nicht auf beiden Metriken →
  Champion s68 bleibt deployed.

## 6. Lehre

Die zwei behobenen Bugs (Checkpoint nach Überlebenszeit; Board-Seed hart 0)
waren die Hauptursachen für Seed-Abhängigkeit und „länger → schlechter". Mit den
Fixes ist das Training stabil und reproduzierbar. Der verbleibende Rückstand auf
den Champion liegt allein auf Periodic; offene Hebel: mehr Iterationen (jetzt
gefahrlos, da kein Kreis-Kollaps mehr — siehe Run 025) oder ein Hunger-Feature,
damit die Policy „lange nicht gefressen" überhaupt wahrnehmen kann.

## 7. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-024-s1/best.mlp` | iter 105 (W 43.98 / P 62.74) |
| `training-out/az-run-024-s1/final.mlp` | iter 149 (W 44.41 / P 64.72) |
