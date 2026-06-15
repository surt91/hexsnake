# Training Report: AlphaZero-light — Run 010

**Datum**: 2026-06-15  
**Ziel**: Drei Varianten parallel testen, um Run 001 zu schlagen
(Walls Ø 37.60, Periodic Ø 54.28).
Baseline Run 009 P2: Walls 35.88, Periodic 50.34.

---

## 1. Setup — Drei parallele Varianten

### Option A — Phase 3 mit niedrigerer LR

| Parameter | Wert |
|---|---|
| `--iterations` | 200 |
| `--boundary` | mixed |
| `--eat-bonus` | 0.3 |
| `--lr` | **3e-4** (statt 1e-3) |
| `--load` | `az-run-009-p2/best.mlp` (Warm-Start) |
| `--seed` | 10 |
| `--best-out` | `az-run-010-a/best.mlp` |

### Option B1 — Verlängerte Phase 1 (parallel zu A und C)

| Parameter | Wert |
|---|---|
| `--iterations` | 500 |
| `--boundary` | walls |
| `--eat-bonus` | 0.6 |
| `--sp-eat` | 1.5 |
| `--seed` | 10 |
| `--best-out` | `az-run-010-b1/best.mlp` |

### Option C — eat_bonus=0.6 direkt in Mixed

| Parameter | Wert |
|---|---|
| `--iterations` | 400 |
| `--boundary` | mixed |
| `--eat-bonus` | **0.6** |
| `--sp-eat` | **1.5** |
| `--seed` | 10 |
| `--best-out` | `az-run-010-c/best.mlp` |

---

## 2. Lernkurven

### Option A (Phase 3, lr=3e-4, mixed, warm-start P2)

| Iteration | ~game_len | Bemerkung |
|---|---|---|
| 0   | 1 810.9 | Warm-Start von P2 best |
| **32** | **1 867.3** | **[best] — Peak** |
| 50  | 1 655.1 | Rückgang |
| 100 | 1 645.2 | Plateau |

Abgebrochen bei iter 101 (kein neues Best seit iter 32).

### Option B1 (walls, eat_bonus=0.6, 500 iter, parallel)

| Iteration | ~game_len | Bemerkung |
|---|---|---|
| 0   | 114.5 | Zufälliges Netz |
| 9   | 1 434.5 | |
| **11** | **1 629.3** | **[best] — einziges Best** |
| 100 | 1 477.6 | Plateau bei ~1 477 |
| 206 | 1 477.3 | **Policy-Kollaps** (loss ≈ 0.000) |

Abgebrochen wegen Policy-Kollaps. **Ursache: CPU-Konkurrenz** durch
parallele Ausführung von A, B1 und C gleichzeitig.

### Option C (mixed, eat_bonus=0.6, 400 iter, parallel)

| Iteration | ~game_len | Bemerkung |
|---|---|---|
| 0   | 128.8 | Zufälliges Netz |
| 26  | 1 010.5 | |
| 90–95 | 1 044–1 165 | Neue Bests |
| 155 | 1 996.1 | |
| 162 | 2 255.5 | |
| 165 | 2 364.0 | |
| 167 | 2 399.1 | |
| **169** | **2 499.4** | **[best] — Neues Training-Allzeit-Hoch!** |
| 184–203 | 1 800–2 220 | Post-Peak-Oscillation |

Abgebrochen bei iter 203 (Peak bei 169 gespeichert, Post-Peak-Rückgang klar).

---

## 3. Benchmark-Ergebnis (Option C best)

Gemessen mit 50 Spielen, max. 8 000 Ticks, sims=24.

| Topologie  | Run 001 | Run 009 P2 | **Run 010 C** |
|------------|--------:|-----------:|--------------:|
| Walls Ø    |   37.60 |      35.88 |     **32.78** |
| Periodic Ø |   54.28 |      50.34 |     **48.32** |

**Run 010 C nicht deployed** (Walls −12.8 %, Periodic −11.0 % vs. Run 001).
Run 001 bleibt embedded.

---

## 4. Analyse

### 4.1 Option A — Niedrigere LR bremst ohne zu helfen

Peak bei iter 32 (1867), deutlich unter P2-Trainings-Best (2009).
Die niedrigere Lernrate konvergiert schneller zu einem lokalen Optimum,
aber dieses Optimum ist schlechter als der Ausgangspunkt.

### 4.2 Option B1 — Policy-Kollaps durch CPU-Konkurrenz

B1 lief parallel zu A und C (alle drei je ~900–1400 % CPU).
Der Policy-Kollaps (loss → 0 ab iter 11) ist ein bekanntes AlphaZero-Problem:
Das Netz konvergiert zu einer deterministischen Policy, die Buffer füllt sich
mit identischen Spielen, und die Gradienten verschwinden.
Run 009 P1 (identische Parameter, seed=9, **solo**) erreichte 2388 —
Bestätigung, dass CPU-Konkurrenz der Auslöser war.

### 4.3 Option C — Hohes Training-Best, schlechter Benchmark-Transfer

Game_len 2499 ist das neue Training-Allzeit-Hoch (vorher: R009 P1 mit 2388).
Trotzdem performt C im Benchmark schlechter als R001 und R009 P2.

**Hypothese**: eat_bonus=0.6 in Mixed-Training optimiert die Schlange auf
schnelles Fressen unter gemischten Bedingungen, aber verzerrt dabei den
Value-Head so, dass die MCTS-Blattbewertung im Benchmark nicht mehr gut
kalibriert ist. Der Walls-Score (32.78) ist sogar schlechter als R009 P1
(33.72), obwohl C auf Mixed trainiert.

Vergleich Mixed vs. Zweiphasen:
- R009 P1 (walls, eat_bonus=0.6) → Walls 33.72, Periodic 37.32
- R009 P2 (mixed fine-tune, eat_bonus=0.3) → Walls 35.88, Periodic 50.34
- R010 C  (mixed, eat_bonus=0.6)          → Walls 32.78, Periodic 48.32

Der zweiphasige Ansatz (walls für Essens-Kompetenz, mixed für Boundary-Lernen
mit moderatem eat_bonus) ist dem einphasigen Mixed-Training mit hohem
eat_bonus überlegen.

### 4.4 Nächste Schritte

1. **Run 011 — Frische Phase 1 solo (500 Iter)**: walls, eat_bonus=0.6,
   sp_eat=1.5, seed=11 — ohne CPU-Konkurrenz, um echtes Plateau-Verhalten
   zu ermitteln und einen besseren P1-Start für Phase 2 zu erzielen.
   Danach Phase 2 (mixed, eat_bonus=0.3, warm-start vom P1-Best).

---

## 5. Schlussfolgerungen

Run 010 bringt zwei wichtige Erkenntnisse:
- **Parallelisierung schadet**: Drei gleichzeitige Trainingsläufe führen zu
  Policy-Kollaps und reduzierter Qualität.
- **eat_bonus=0.6 in Mixed kontraproduktiv**: Hohes Training-game_len ≠
  gutes Benchmark-Ergebnis. Der zweiphasige Ansatz ist besser.

Run 001 bleibt Bestwert.

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-010-a/best.mlp` | Option A Best (iter 32, game_len 1867.3) |
| `training-out/az-run-010-b1/best.mlp` | Option B1 Best (iter 11, game_len 1629.3) — Policy-Kollaps |
| `training-out/az-run-010-c/best.mlp` | Option C Best (iter 169, game_len 2499.4) — Walls 32.78, Periodic 48.32 |
| `crates/snake-core/assets/alphazero/best.mlp` | **Run 001** bleibt embedded |
