# Training Report: AlphaZero-light — Run 005

**Datum**: 2026-06-14  
**Ziel**: Run-001-Reward + max-ticks=4000 (kein Zeitlimit-Clipping).
Hypothese: das ursprüngliche eat=0.3 war gut, aber max-ticks=1500 limitierte
welche Brettzustände das Netz je sah.
Baseline: Run 001 (Walls Ø 37.60, Periodic Ø 54.28).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | AlphaZero-light Gradient |
| `--iterations` | 300 |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--temperature` | 1.0 |
| `--epochs` | 4 |
| `--boundary` | mixed (50/50 Walls/Torus) |
| `--max-ticks` | **4 000** |
| `--seed` | 5 |

| Reward-Parameter | Wert |
|---|---|
| MCTS eat-Bonus | 0.3 (Run-001-Wert) |
| Self-Play Fressen | +1.0 (Run-001-Wert) |
| Self-Play Lebenskosten | −0.005 (Run-001-Wert) |

---

## 2. Lernkurve

| Iteration | Policy-Loss | ~game_len | Bemerkung |
|---|---|---|---|
| 0   | 1.570 |   160 | Zufälliges Netz |
| 23  | 0.106 |   602 | |
| 48  | 0.028 |   848 | |
| 73  | 0.007 | 1 323 | Erster Peak |
| 98  | 0.009 | 1 109 | Rückgang |
| 148 | 0.019 | 1 149 | Stagnation |
| 198 | 0.002 | 1 977 | Zweiter Anstieg |
| 223 | 0.000 | **2 529** | **Globaler Peak** |
| 248 | 0.033 | 1 885 | Rückgang |
| 273 | 0.034 | 1 188 | Starker Rückgang |
| 299 | 0.001 | 1 463 | Endwert — noch nicht konvergiert |

**Entscheidende Beobachtung**: Das Training konvergiert nicht stabil.
game_len schwankt zwischen 956 und 2529 — Run 005 war bei 300 Iterationen
noch nicht fertig. Das globale Peak bei iter 223 (game_len 2529) wurde
nicht gespeichert (kein Checkpoint); der exportierte Wert ist iter 299 (1463).

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks, sims=24.

| Topologie  | Run 001 | **Run 005** | Δ |
|------------|--------:|------------:|--:|
| Walls Ø    |   37.60 |   **24.10** | −35.9 % |
| Periodic Ø |   54.28 |   **56.04** | **+3.2 %** |

**Erstmals** schlägt ein Run run-001 auf Periodic. Max-Score auf Walls: 86
(vs 61 in Run 001) — einzelne Spiele sind sehr stark, aber die Varianz ist hoch.

**Run 001 bleibt deployed** (besseres Gesamtergebnis).

---

## 4. Analyse

### 4.1 Positiv: max-ticks=4000 wirkt

Das Netz lernt erstmals lange Schlangen-Zustände (game_len bis 2529 im Training,
vs. run-003 mit game_len immer bei 1477/1500). Periodic verbessert sich dadurch
auf 56.04 (+3.2 %). Der max-Score auf Walls steigt auf 86 (+40 % vs run-001).

### 4.2 Negativ: 300 Iterationen zu wenig

Die Kurve hat bei iter 223 ein starkes Peak und fällt danach auf 1188 zurück.
Das ist kein Plateau, sondern ein Überschwingen — die Policy war noch im
aktiven Lernprozess. Der exportierte Checkpoint (iter 299, game_len=1463) ist
nicht der beste Trainingszustand.

### 4.3 Walls vs. Periodic

Run 005 verbessert Periodic (+3.2 %) aber verschlechtert Walls (−35.9 %).
Die hohe Walls-avg-Ticks-Zahl (3513) zeigt: die Schlange überlebt sehr lange,
isst aber ineffizient. Das Muster des „sicheren Kreisens" ist noch vorhanden
auf Walls, aber auf Periodic (wo Randüberschreitungen sicherer sind) kann die
Schlange freier agieren.

---

## 5. Schlussfolgerungen

**Run 006** (bereits gestartet): Gleiche Parameter, aber 600 statt 300 Iterationen.
Mit dem stabilen Peak-Muster ab iter ~100 und dem zweiten Anstieg ab iter ~175
sollte ein längerer Lauf beide Metriken verbessern.

Weitere Optionen:
- Checkpoint-Speicherung implementieren (best model per game_len, nicht nur letzter)
- Learning-Rate-Schedule (LR decay ab iter ~200)

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-005/best.mlp` | Gewichte Run 005 (nicht deployed) |
| `training-out/az-run-005/train.log` | Lernkurve (300 iter) |
| `crates/snake-core/assets/alphazero/best.mlp` | **Run 001** bleibt embedded |
