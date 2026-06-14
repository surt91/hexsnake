# Training Report: AlphaZero-light — Run 004

**Datum**: 2026-06-14  
**Ziel**: Training-Distribution-Mismatch aus Run 003 beheben: max-ticks 1500 → 4000,
damit das Netz auch lange Schlangen-Situationen sieht.
Baseline: Run 001 (Walls Ø 37.60, Periodic Ø 54.28).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | AlphaZero-light Gradient |
| `--iterations` | 200 |
| `--games-per-iter` | 64 |
| `--sims` | 16 |
| `--temperature` | 1.0 |
| `--epochs` | 4 |
| `--boundary` | mixed (50/50 Walls/Torus) |
| `--max-ticks` | **4 000** |
| `--seed` | 4 |
| Architektur | 20→32→24→7 |

**Motivation**: Run 003 (max-ticks=1500) zeigte game_len=1477 — die Schlange
traf fast immer das Zeitlimit. Der Value-Head wurde nie auf lange Schlangen
kalibriert. Durch max-ticks=4000 soll das Netz echte End-Game-Situationen lernen.

---

## 2. Lernkurve

| Iteration | Policy-Loss | ~game_len | Bemerkung |
|---|---|---|---|
| 0   | 1.601 |   105 | Zufälliges Netz |
| 18  | 0.097 |   412 | |
| 38  | 0.033 |   457 | |
| 58  | 0.054 |   676 | |
| 78  | 0.002 |   833 | Policy konvergiert |
| 98  | 0.013 |   982 | |
| 118 | 0.002 | 1 033 | Plateau |
| 158 | 0.000 | 1 036 | |
| 199 | 0.001 | **1 040** | Stabil, weit unter max-ticks=4000 |

**Entscheidender Unterschied zu Run 003**: game_len stabilisiert bei ~1040 von 4000 max
(26 %). Die Schlange stirbt jetzt natürlich — kein Zeitlimit-Clipping mehr.
Der Value-Head wird auf echte Spielverläufe kalibriert.

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks, sims=24 (embedded).

| Topologie  | Run 001 | **Run 004** | Δ |
|------------|--------:|------------:|--:|
| Walls Ø    |   37.60 |   **24.76** | −34.1 % |
| Periodic Ø |   54.28 |   **43.86** | −19.2 % |

**Regression trotz besserer Trainingsvoraussetzungen.** Run 001 bleibt deployed.

---

## 4. Analyse: Das eat=1.0 Paradoxon

Alle Runs mit erhöhtem eat-Bonus (0.5, 1.0) zeigen im Benchmark weniger
Fressen als Run 001 (eat=0.3):

| Run | MCTS eat | game_len Train | Walls Score | Ticks/Apfel (Benchmark) |
|-----|--------:|---------------:|------------:|------------------------:|
| 001 | 0.3 | ~787 | **37.60** | ~36 |
| 002 | 0.5 | ~658 | 19.58 | ~228 |
| 003 | 1.0 | ~1477 | 22.58 | ~198 |
| 004 | 1.0 | ~1040 | 24.76 | ~63 |

Höherer eat-Bonus → mehr Fressen **im Training** (MCTS jagt Futter) →
aber weniger Fressen im **Benchmark** (destillierte Policy generalisiert
nicht). Die Policy lernt vom MCTS, nicht direkt von der Reward-Funktion.

**Hypothese**: Mit eat=1.0 dominiert das Fress-Signal die MCTS-Besuchsverteilung
so stark, dass die Policy nur sehr spezifische „Futter-ist-hier-rechts"-Muster
lernt. Diese Muster sind zu spezifisch für die Trainings-Seeds und generalisieren
nicht auf neue Spielverläufe. Run 001 mit eat=0.3 zwang die Policy, robustere
allgemeine Navigationsstrategien zu lernen.

---

## 5. Schlussfolgerungen

**Reverte eat-Bonus auf Run-001-Werte (0.3)** für zukünftige Runs.

Die bisherigen Erkenntnisse zusammengefasst:
- ✅ max-ticks=4000 ist besser als 1500 (kein Zeitlimit-Clipping)
- ✅ Mehr Iterationen (> 80) bringen prinzipiell mehr Kapazität
- ❌ eat=0.5 und eat=1.0 schaden beide
- ❌ Mehr Budget (2000 iter) ohne early-stopping konvergiert zu schlechten Lösungen

**Für Run 005** empfohlen:
- MCTS eat=0.3 (Run-001-Wert)
- Self-Play eat=1.0, living=-0.005 (Run-001-Werte)
- max-ticks=4000 (neu — verhindert Zeitlimit-Clipping)
- iterations=200–400, early-stopping am Peak
- games=128, sims=24

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-004/best.mlp` | Gewichte Run 004 (nicht deployed) |
| `training-out/az-run-004/train.log` | Lernkurve (200 iter) |
| `crates/snake-core/assets/alphazero/best.mlp` | **Run 001** bleibt embedded |
