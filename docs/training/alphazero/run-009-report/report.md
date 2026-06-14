# Training Report: AlphaZero-light — Run 009

**Datum**: 2026-06-15  
**Ziel**: Zweiphasiges Training mit erhöhtem eat_bonus in Phase 1, um das
„sichere Kreisen"-Lokaloptimum aus Run 008 zu vermeiden.
Phase 1: walls-only, eat_bonus=0.6 (statt 0.3), kurz (300 Iter).
Phase 2: mixed Fine-Tuning, eat_bonus=0.3, Warm-Start vom Phase-1-Best.
Baseline: Run 001 (Walls Ø 37.60, Periodic Ø 54.28).

---

## 1. Setup

### Phase 1 — Walls-only, erhöhter eat-Bonus (300 Iterationen)

| Parameter | Wert |
|---|---|
| `--iterations` | 300 |
| `--boundary` | walls |
| `--eat-bonus` | **0.6** (statt Standardwert 0.3) |
| `--sp-eat` | **1.5** (statt 1.0) |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--max-ticks` | 4 000 |
| `--seed` | 9 |
| `--best-out` | `az-run-009-p1/best.mlp` |

### Phase 2 — Mixed Fine-Tuning (400 Iterationen)

| Parameter | Wert |
|---|---|
| `--iterations` | 400 |
| `--boundary` | mixed |
| `--eat-bonus` | 0.3 (Standardwert) |
| `--sp-eat` | 1.0 (Standardwert) |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--max-ticks` | 4 000 |
| `--seed` | 9 |
| `--load` | `az-run-009-p1/best.mlp` (Warm-Start) |
| `--best-out` | `az-run-009-p2/best.mlp` |

---

## 2. Lernkurve

### Phase 1 (walls, eat_bonus=0.6)

| Iteration | ~game_len | Bemerkung |
|---|---|---|
| 0   |   102.0 | Zufälliges Netz |
| 50  | 1 195.0 | Schnelles Lernen |
| 100 | 1 484.1 | |
| 150 | 1 995.2 | |
| **198** | **2 388.0** | **[best] — Peak** |
| 250 | 2 210.4 | Leichter Rückgang |
| 299 | ~ 2 200 | Endwert |

**Vergleich mit Run 008 Phase 1** (eat_bonus=0.3): Peak 1 386 bei iter 193.
Eat_bonus=0.6 verdoppelt den Peak-game_len (2 388 vs. 1 386) — **kein Plateau** mehr.

### Phase 2 (mixed, eat_bonus=0.3)

| Iteration | ~game_len | Bemerkung |
|---|---|---|
| 0   | 1 396.5 | Warm-Start von P1 Best |
| 25  |   976.0 | Buffer-Einbruch (neue Boundary-Daten) |
| 100 | 1 472.2 | Erholung |
| 150 | 1 762.5 | |
| **179** | **2 009.1** | **[best] — Peak** |
| 200 | 1 636.0 | Rückgang |
| 300 | 1 254.9 | |
| 375 |   988.3 | Endwert — starker Rückgang |

Peak bei iter 179 (2009), dann erneute Oszillation nach unten.

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks, sims=24.

| Topologie  | Run 001 | Run 008 P2 | Run 007 | **Run 009 P1** | **Run 009 P2** |
|------------|--------:|-----------:|--------:|---------------:|---------------:|
| Walls Ø    |   37.60 |      28.30 |   30.16 |          33.72 |      **35.88** |
| Periodic Ø |   54.28 |      12.88 |   59.18 |          37.32 |      **50.34** |
| Ø Ticks (Walls)    | — | 900 | — | 1 997 | 1 874 |
| Ø Ticks (Periodic) | — | 7 137 | — |  829 |  1 471 |

**Run 009 ist der bisher beste Zweiphasen-Run** — deutliche Verbesserung
gegenüber Run 008. Noch unter Run 001, aber viel näher dran.

---

## 4. Analyse

### 4.1 eat_bonus=0.6 behebt das Plateauproblem

| Run | eat_bonus | Phase-1-Peak game_len | Walls P1-Bench | Periodic P1-Bench |
|-----|----------:|----------------------:|---------------:|------------------:|
| 008 | 0.3 | 1 386 (iter 193) | 26.44 | 20.98 |
| **009** | **0.6** | **2 388 (iter 198)** | **33.72** | **37.32** |

Der höhere eat-Bonus verdoppelt den Phase-1-Peak-game_len. Wichtiger noch:
Der Phase-1-Value-Head ist jetzt **nicht mehr rein ums Überleben** kalibriert —
Ø Ticks auf Periodic ist jetzt 829 statt 7 137. Die Schlange sucht aktiv Futter.

### 4.2 Phase 2 verbessert beide Metriken

P1 best → P2 best:
- Walls: 33.72 → **35.88** (+6.4 %)
- Periodic: 37.32 → **50.34** (+34.9 %)

Phase 2 verbessert Periodic dramatisch (Warm-Start lernt schnell die Torus-Grenzen).
Walls steigt ebenfalls weiter (der Warm-Start-Vorteil von Phase 1 bleibt erhalten).

### 4.3 Phase-2-Oscillation bleibt das Problem

Wie in allen bisherigen Runs fällt game_len nach dem Peak (iter 179) steil ab.
Der Peak-Checkpoint wird korrekt gespeichert, aber eine längere Phase 2 würde
keinen weiteren Anstieg bringen — nur mehr Varianz.

**Hypothese für den Walls-Rückstand**: Die besten Walls-Scores kommen bei hohem
game_len in der Phase-1-Training (Schlange lebt lange). In Phase 2 lernt die
Policy, mit Torus-Grenzen umzugehen, was die Walls-Taktik leicht stört.

### 4.4 Nächste Schritte

Die Kurve legt nahe, dass mehr Phase-2-Budget kaum hilft (Oscillation ab iter 200).
Interessantere Optionen:

1. **Phase 3 mit niedrigerer LR** (`--lr 3e-4`): Vom Phase-2-Best warm-starten
   und mit niedrigerer Lernrate stabilisieren — verhindert das Überschreiben
   des Peaks.

2. **Längere Phase 1** (500–600 Iter mit eat_bonus=0.6): Phase 1 endet noch nicht
   am echten Plateau; ein höherer Phase-1-Peak könnte den Phase-2-Ausgangspunkt
   verbessern.

3. **eat_bonus=0.6 direkt in mixed Training** (ohne Zweiphasen-Ansatz): Testen,
   ob eat_bonus=0.6 auch in purem Mixed-Training besser ist als 0.3.

---

## 5. Schlussfolgerungen

**Run 009 P2 nicht deployed** (Walls −4.6 %, Periodic −7.3 % vs. Run 001).
Run 001 bleibt embedded.

Der zweiphasige Ansatz mit eat_bonus=0.6 funktioniert deutlich besser als
mit 0.3 (Run 008). Die Lücke zu Run 001 ist auf 4–7 % geschrumpft — ein
klarer Fortschritt. Die nächste vielversprechendste Variante ist eine Phase 3
mit niedrigerer Lernrate.

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-009-p1/best.mlp` | Phase 1 Best (iter 198, game_len 2388.0) — Walls 33.72, Periodic 37.32 |
| `training-out/az-run-009-p1/final.mlp` | Phase 1 Final (iter 299) |
| `training-out/az-run-009-p1/train.log` | Lernkurve Phase 1 (300 iter) |
| `training-out/az-run-009-p2/best.mlp` | Phase 2 Best (iter 179, game_len 2009.1) — Walls 35.88, Periodic 50.34 |
| `training-out/az-run-009-p2/final.mlp` | Phase 2 Final (iter 399) |
| `training-out/az-run-009-p2/train.log` | Lernkurve Phase 2 (400 iter) |
| `crates/snake-core/assets/alphazero/best.mlp` | **Run 001** bleibt embedded |
