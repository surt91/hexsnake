# Training Report: AlphaZero-light — Run 012

**Datum**: 2026-06-15  
**Ziel**: Run 001 mit max-ticks=1500 reproduzieren und mit mehr Iterationen
(150 statt 80) verbessern. Seed=12 (neu), alle anderen Parameter identisch
zu Run 001.
Baseline: Run 001 (Walls Ø 37.60, Periodic Ø 54.28).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| `--iterations` | 150 |
| `--boundary` | mixed |
| `--eat-bonus` | 0.3 (Standard, wie Run 001) |
| `--max-ticks` | **1 500** (wie Run 001) |
| `--lr` | 1e-3 |
| `--seed` | **12** (neu) |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--best-out` | `az-run-012/best.mlp` |

---

## 2. Lernkurve

| Iteration | ~game_len | Bemerkung |
|---|---|---|
| 0   |  157.7 | Zufälliges Netz |
| 77  |  841.8 | Vergleichbar Run 001 (iter 79: ~787) |
| 134 | 1 086.4 | [best] |
| 140 | 1 090.6 | [best] |
| 141 | 1 108.3 | [best] |
| 142 | 1 125.7 | [best] |
| 143 | 1 148.6 | [best] |
| **145** | **1 159.0** | **[best] — Peak** |
| 149 | 1 150.0 | Endwert (Lauf fertig) |

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks, sims=24.

| Topologie  | Run 001 | **Run 012** | Δ |
|------------|--------:|------------:|--:|
| Walls Ø    |   37.60 |   **31.52** | −16.2 % |
| Periodic Ø |   54.28 |   **55.70** | **+2.6 %** |

**Run 012 nicht deployed.** Walls deutlich schlechter als Run 001.
Periodic marginal besser (+2.6 %).

---

## 4. Analyse

### 4.1 max-ticks=1500 allein reicht nicht

Run 012 hat dieselben Hyperparameter wie Run 001, nur seed=12 statt seed=1
und 150 statt 80 Iterationen. Trotzdem schlechtere Walls-Leistung.

Der Vergleich bei Iteration 77:
- Run 001 (seed=1): game_len ~787
- Run 012 (seed=12): game_len ~841

Ähnliche game_len-Trajektorie — aber anderes Benchmark-Ergebnis.

### 4.2 Seed-Abhängigkeit ist stark

Run 001 mit seed=1 bei 80 Iter → Walls 37.60.
Run 012 mit seed=12 bei 150 Iter → Walls 31.52.
Run 003 mit max-ticks=1500 und eat_bonus=1.0 → Walls 22.58 (Sättigungs-Kollaps).

Die Walls-Metrik variiert um >35 % zwischen Seeds. Die bestimmende Variable
für die hohe Walls-Leistung von Run 001 ist der **Seed=1**, nicht nur
max-ticks=1500 oder die Iterationszahl.

### 4.3 Zu viele Iterationen schaden Walls

Run 012 continued past Run 001's stopping point (iter 79/787 game_len).
Bei iter 145 (game_len 1159 — 76% des 1500-Deckels) war Walls deutlich
schlechter. Der optimale Punkt für Walls scheint früher zu liegen
(game_len ~787, 52% des Deckels).

**Hypothese**: Der Value-Head wird bei hoher game_len (>50% des Deckels)
auf „lange überleben" kalibriert — ähnlich wie bei max-ticks=4000, nur
milder. Run 001 stopped im „Fress-optimalen" Fenster.

### 4.4 Nächste Schritte — Run 013

Test: seed=1 (identisch Run 001) mit 150 Iterationen. Wenn der Best-
Checkpoint bei iter ~79 liegt (wie Run 001), bestätigt das:
1. Reproduzierbarkeit von Run 001
2. Ob mehr Iterationen mit seed=1 helfen oder schaden

---

## 5. Schlussfolgerungen

max-ticks=1500 ist notwendig aber nicht hinreichend. Run 001 kombiniert
max-ticks=1500 mit seed=1 in einem zufällig guten „Sweet Spot". Weitere
Runs müssen seed=1 und frühe Stopping-Kriterien testen.

Run 001 bleibt embedded.

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-012/best.mlp` | Best (iter 145, game_len 1159.0) — Walls 31.52, Periodic 55.70 |
| `training-out/az-run-012/final.mlp` | Final (iter 149) |
| `training-out/az-run-012/train.log` | Lernkurve (150 Iter) |
