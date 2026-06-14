# Training Report: AlphaZero-light — Run 006

**Datum**: 2026-06-14  
**Ziel**: Run-005 verlängern: gleiche Parameter, aber 600 statt 300 Iterationen.
Hypothese: mehr Budget überwindet den Mid-Training-Dip und konsolidiert den Peak
bei iter 223 (game_len 2529 in Run 005).
Baseline: Run 001 (Walls Ø 37.60, Periodic Ø 54.28).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | AlphaZero-light Gradient |
| `--iterations` | 600 |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--temperature` | 1.0 |
| `--epochs` | 4 |
| `--boundary` | mixed (50/50 Walls/Torus) |
| `--max-ticks` | **4 000** |
| `--seed` | 6 |

| Reward-Parameter | Wert |
|---|---|
| MCTS eat-Bonus | 0.3 (Run-001-Wert) |
| Self-Play Fressen | +1.0 (Run-001-Wert) |
| Self-Play Lebenskosten | −0.005 (Run-001-Wert) |

---

## 2. Lernkurve

| Iteration | Policy-Loss | ~game_len | Bemerkung |
|---|---|---|---|
| 0   | 1.570 |   159.7 | Zufälliges Netz |
| 50  | 0.022 |   776.3 | Schnelles Lernen |
| 100 | 0.019 |   990.7 | |
| 200 | 0.001 | 1 917.4 | Zweiter Anstieg |
| 300 | 0.012 | 1 411.6 | Rückgang |
| 400 | 0.007 | 2 682.3 | Dritter Anstieg |
| 426 | 0.000 | **2 779.6** | **Globaler Peak** |
| 500 | 0.001 | 1 693.4 | Starker Rückgang |
| 599 | 0.013 |   961.1 | Exportierter Endwert |

**Entscheidende Beobachtung**: game_len oszilliert mit Periode ~100–200 Iterationen.
Der globale Peak bei iter 426 (game_len 2779.6) ist deutlich besser als der
exportierte Endwert (iter 599, game_len 961.1). Da kein Checkpoint-Speichern
implementiert war, ging der beste Trainingszustand verloren.

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks, sims=24 (embedded).

| Topologie  | Run 001 | **Run 006** | Δ |
|------------|--------:|------------:|--:|
| Walls Ø    |   37.60 |   **27.94** | −25.7 % |
| Periodic Ø |   54.28 |   **50.88** |  −6.3 % |

**Regression.** Ursache: der exportierte Endwert (iter 599, game_len=961) ist
weit vom Peak entfernt. Run 001 bleibt deployed.

---

## 4. Analyse: Das Checkpoint-Problem

### 4.1 Ursache der Oszillation

Run 006 zeigt die Oszillation aus Run 005 noch deutlicher: drei ausgeprägte Peaks
bei iter ~100 (990), ~200 (1917), ~426 (2779), dann ein stetiger Rückgang.

Die Ursache ist vermutlich eine Interaktion zwischen Replay-Buffer und Lernrate:
Wenn das Netz sich verbessert, erzeugt die Self-Play qualitativ andere Daten als
die alten Buffer-Einträge. Das Netz überschreibt gute Gewichte mit einer Policy,
die auf schlechtere alte Daten zugeschnitten ist.

### 4.2 Kein Checkpoint → kein Nutzen aus Peak

Das exportierte Modell entspricht dem Netz am Ende der 600 Iterationen — nicht dem
besten Trainingszustand. Der Peak bei iter 426 (game_len 2779.6) wäre ca. 189 %
besser als der Endwert gewesen. Ein Benchmark des Peak-Checkpoints hätte
wahrscheinlich run-001 auf Periodic übertroffen (vgl. Run 005 Peak → +3.2 %).

### 4.3 Vergleich Run 005 vs. Run 006

| Metrik | Run 005 | Run 006 |
|---|---|---|
| Iterationen | 300 | 600 |
| Peak game_len | 2 529 (iter 223) | **2 779** (iter 426) |
| Peak iter / max iter | 74 % | 71 % |
| Endwert game_len | 1 463 | 961 |
| Walls Benchmark | 24.10 | 27.94 |
| Periodic Benchmark | **56.04** | 50.88 |

Run 006 hat einen höheren Peak als Run 005, aber einen schlechteren Endwert.
Mehr Iterationen ohne Checkpoint-Speicherung sind kontraproduktiv.

---

## 5. Schlussfolgerungen

**Checkpoint-Speicherung ist obligatorisch**: Das nächste Training muss das beste
Modell (nach game_len) zwischenspeichern. Dieses Feature wurde nach diesem Run in
`python/train_alphazero.py` implementiert (`--best-out`-Flag, `[best]`-Marker im Log).

Weitere Erkenntnisse:
- ✅ Höherer Peak mit mehr Budget (2779 vs 2529)
- ✅ Peak liegt reproduzierbar bei ~70 % der Iterationen
- ❌ Endwert nach dem Peak sinkt bei längeren Runs stärker
- ❌ Ohne Checkpoint geht der Peak verloren

**Für Run 007** (mit Checkpoint-Fix):
- Gleiche Parameter wie Run 005/006
- `--best-out az-run-007/best.mlp` sichert automatisch den Peak
- 600 Iterationen (Peak lag bisher bei ~70 %)

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-006/best.mlp` | Gewichte Run 006 (Endwert iter 599, **nicht deployed**) |
| `training-out/az-run-006/train.log` | Lernkurve (600 iter) |
| `crates/snake-core/assets/alphazero/best.mlp` | **Run 001** bleibt embedded |
