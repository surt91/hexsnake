# Training Report: AlphaZero-light — Run 026 (Hunger-Feature, 6 h)

**Datum**: 2026-06-16
**Ziel**: Ein **Hunger-Feature** (Ticks seit letztem Futter) zum AZ-Input
hinzufügen und mit großem Budget (Ziel 6 h Wall-Clock) trainieren. Hypothese:
Lokale Sensoren können „frisch gefüttert" nicht von „kreist seit langem ohne zu
fressen" unterscheiden — Hunger soll dem Value-Kopf dieses Signal geben.

Referenz (vorheriger Champion): Run 025 Seed 1 (20-Input, ohne Hunger) —
Walls 48.21, Periodic 72.45, Avg 60.33 (200 Spiele).

---

## 1. Implementierung

- **AZ-eigener 21-Feature-Vektor** `az_features` = die geteilten 20 Sensoren +
  Hunger (`ticks_since_food` / Brettfläche, geklippt auf [0,1]). Die geteilte
  `features` (20) und die NEAT/DQN/PPO/MLP-GA-Netze bleiben unangetastet.
- `GameState.ticks_since_food` (Reset beim Fressen).
- Python `AZNet` Input 20 → 21; `--max-hours` Wall-Clock-Budget im Trainer
  (best.mlp wird laufend beim besten Eval gespeichert → zeitbegrenzter Lauf
  liefert trotzdem das beste Netz).

## 2. Setup

| Parameter | Wert |
|---|---|
| `--max-hours` | 6 (→ **21 721 Iterationen**) |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--max-ticks` | 1500 |
| `--seed` | 1 |
| `--eval-every` / `--eval-games` / `--eval-max-ticks` | 5 / 20 / 4000 |
| Checkpoint-Auswahl | greedy Eval-Mittel (Walls+Periodic) |

## 3. Verlauf

Die greedy-Eval **plateaut früh**: bester Eval-Checkpoint iter 9730
(W 44.6 / P 82.2 / avg 63.4) schon nach ~2,5 h; die restlichen ~3,5 h / 12 000
Iterationen bringen kein neues Maximum. `sp_ticks` ~700–800, `eval_ticks` ~900 —
kein Kreis-Kollaps.

## 4. Benchmark (`bench_mlp`, 200 Spiele, 8000 Ticks, sims 24)

| Netz | Walls | Periodic | Avg |
|---|---:|---:|---:|
| **Run 025 (ohne Hunger)** | **48.21** | 72.45 | **60.33** |
| Run 026 best (iter 9730) | 40.59 | **74.61** | 57.60 |
| Run 026 final (iter 21720) | 43.09 | 66.75 | 54.92 |

## 5. Analyse — das Feature lohnt sich (hier) nicht

- **Netto schlechter**: +2.2 Periodic (vermutlich im Rauschen), aber −7.6 Walls
  → −4.5 % im Mittel gegenüber Run 025.
- **Warum?** Der Board-Vielfalt-Fix (Run 024) hatte das Kreisen **bereits**
  beseitigt (Run 025 hat gesunde avg_ticks). Hunger löst also kein offenes
  Problem mehr, sondern verschiebt nur das Risikoprofil: aggressiveres
  Futter-Ansteuern hilft auf dem Torus (keine Wände), tötet aber häufiger an
  Wänden → Walls-Score fällt.
- **Auswahl-Bias**: Das Eval-Mittel `(W+P)/2` mit optimistischem Periodic-Eval
  (~+8–10 vs. Bench) zieht die Auswahl zu Periodic-lastigen Checkpoints. Eine
  balanciertere Auswahl (`min(W,P)` oder 8000-Tick-Eval) hätte evtl. ein
  ausgewogeneres Netz gewählt — aber Zwischen-Checkpoints werden nicht
  gespeichert, daher nicht nachträglich prüfbar.
- **Längeres Training half nicht weiter**: nach ~2,5 h konvergiert; 6 h bringen
  gegenüber ~3 h nichts. Das ist kein Reward-Kollaps mehr (gut), nur Sättigung.

## 6. Lehre / Entscheidung

Die Lehre aus Run 022–025 hält: Der **Board-Seed-Fix** war die eigentliche
Lösung der Seed-Lotterie; das Hunger-Feature ist ein redundanter Zusatz, der die
Balance verschlechtert. Run 025 bleibt das beste Netz.

**Deploy-Entscheidung**: [vom Nutzer — Revert auf Run 025 vs. Run 026 behalten.]

## 7. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-026-s1/best.mlp` | iter 9730, 21-Input (W 40.59 / P 74.61) |
| `training-out/az-run-026-s1/final.mlp` | iter 21720 (W 43.09 / P 66.75) |
