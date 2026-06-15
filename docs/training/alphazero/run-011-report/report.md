# Training Report: AlphaZero-light — Run 011

**Datum**: 2026-06-15  
**Ziel**: Zweiphasiges Training solo (ohne CPU-Konkurrenz), um Policy-Kollaps
aus Run 010 B1 zu vermeiden.
Phase 1: walls-only, eat_bonus=0.6, 500 Iter, seed=11.
Phase 2: mixed Fine-Tuning, warm-start vom P1-Best.
Baseline: Run 001 (Walls Ø 37.60, Periodic Ø 54.28).

---

## 1. Setup

### Phase 1 — Walls-only (500 Iterationen, solo)

| Parameter | Wert |
|---|---|
| `--iterations` | 500 |
| `--boundary` | walls |
| `--eat-bonus` | 0.6 |
| `--sp-eat` | 1.5 |
| `--max-ticks` | 4 000 |
| `--lr` | 1e-3 |
| `--seed` | 11 |
| `--best-out` | `az-run-011-p1/best.mlp` |

### Phase 2 — Mixed Fine-Tuning (400 Iterationen, solo)

| Parameter | Wert |
|---|---|
| `--iterations` | 400 |
| `--boundary` | mixed |
| `--eat-bonus` | 0.3 |
| `--max-ticks` | 4 000 |
| `--lr` | 1e-3 |
| `--seed` | 11 |
| `--load` | `az-run-011-p1/best.mlp` |
| `--best-out` | `az-run-011-p2/best.mlp` |

---

## 2. Lernkurven

### Phase 1 (walls, eat_bonus=0.6, seed=11)

| Iteration | ~game_len | Bemerkung |
|---|---|---|
| 0   |   114.5 | Zufälliges Netz |
| 163 | 2 014.1 | |
| 164 | 2 042.3 | |
| 165 | 2 091.8 | |
| 171 | 2 094.6 | |
| 175 | 2 100.1 | |
| 183 | 2 108.2 | |
| 184 | 2 114.1 | |
| **193** | **2 129.7** | **[best] — Peak** |
| 200–406 | ~1 867 | **Policy-Kollaps** (loss ≈ 0.000) |

Abgebrochen bei iter 406 (best bei iter 193 gespeichert).

**Vergleich mit Run 009 P1** (seed=9, solo): Peak 2 388 bei iter 198.
Run 011 P1 (seed=11) erreicht nur 2 130 — Seed-Abhängigkeit bestätigt.
Trotz Solo-Ausführung tritt Policy-Kollaps erneut auf (~iter 200).

### Phase 2 (mixed, warm-start von P1 best, seed=11)

| Iteration | ~game_len | Bemerkung |
|---|---|---|
| **0** | **3 053.4** | **[best] — Warm-Start, Allzeit-Hoch in Mixed!** |
| 25 | ~ 900 | Buffer-Einbruch (neue Boundary-Daten) |
| 82 | 1 459.4 | Talsohle |
| 102 | 1 636.2 | Erholung |
| 306 | 1 854.1 | Sekundärer Peak |
| 310 | 1 028.4 | Zweiter Einbruch |

Abgebrochen bei iter 310 (best bleibt iter 0, kein weiteres Best-Update).

**Besonderheit**: game_len 3 053 bei iter 0 ist das höchste je gemessene
Trainings-game_len auf Mixed-Boundaries. Das walls-trainierte Netz spielt
auf Torus sehr lang, bevor der Value-Head für Mixed rekalibriert wird.
Der Post-Einbruch-Peak (iter 306: 1 854) ist niedriger als Run 009 P2
(iter 179: 2 009) — weil der P1-Startpunkt (2 130) schlechter war als
R009 P1 (2 388).

---

## 3. Benchmark-Ergebnis

**Nicht gemessen** — Run 011 P2 best (iter 0, game_len 3 053) entspricht
dem P1-Netz ohne Mixed-Rekalibrierung. Der nachfolgende Phase-2-Peak
(1 854 bei iter 306) ist niedriger als R009 P2 (2 009) und damit
voraussichtlich schlechter als R009 P2 (Walls 35.88, Periodic 50.34).
Kein Benchmark durchgeführt.

---

## 4. Analyse

### 4.1 max-ticks=4000 als Kern des Problems

Während des Reportings wurde ein entscheidender Fehler aus den frühen
Reports identifiziert:

**Run 001 (Walls 37.60, Periodic 54.28 — beste bisherige Ergebnisse)**
trainierte mit `--max-ticks 1500`. Alle Runs ab Run 004 nutzen
`--max-ticks 4000`.

Mit max-ticks=4000:
- Hohe game_len (2000–3000 Ticks) entsteht durch sicheres Kreisen
- Der Value-Head lernt „lange überleben" statt „effizient fressen"
- Im Benchmark (max-ticks=8000) kreist das Netz ebenfalls statt zu jagen

Mit max-ticks=1500 (Run 001):
- game_len ~800 → Schlange frißt aktiv (weit unter Zeitlimit)
- Value-Head kalibriert auf Fresseffizienz
- Benchmark: besseres Score-zu-Ticks-Verhältnis

### 4.2 Policy-Kollaps ist seed-abhängig

| Lauf | Seed | P1-Peak | Kollapszeitpunkt |
|------|-----:|--------:|-----------------:|
| R009 P1 | 9 | 2 388 (iter 198) | nach iter 250 |
| R011 P1 | 11 | 2 130 (iter 193) | nach iter 200 |

Der Kollaps tritt in ähnlicher Iteration auf, aber der Peak-Wert variiert
stark mit dem Seed.

### 4.3 Nächste Schritte — Run 012

**Hypothese für Run 012**: Run 001 war erfolgreich weil max-ticks=1500
Fresseffizienz erzwingt. Mit mehr Iterationen als Run 001 (80) aber
Stopp bevor Sättigung:

- boundary=mixed, max-ticks=**1500**, eat_bonus=0.3 (wie Run 001)
- 150 Iterationen (mehr als Run 001, weniger als Sättigungsgrenze)
- Seed=12
- Ziel: game_len ~1000–1200 (unter Deckel 1500)

---

## 5. Schlussfolgerungen

Run 011 wiederholt Run 009 mit schlechterem Seed und bestätigt die
Policy-Kollaps-Problematik. Der entscheidende Befund: **max-ticks=4000
ist der Hauptgrund warum alle Runs seit 004 schlechter als Run 001 sind**.

Run 001 bleibt embedded.

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-011-p1/best.mlp` | Phase 1 Best (iter 193, game_len 2129.7) |
| `training-out/az-run-011-p1/train.log` | Lernkurve Phase 1 (407 Zeilen) |
| `training-out/az-run-011-p2/best.mlp` | Phase 2 Best (iter 0, game_len 3053.4) |
| `training-out/az-run-011-p2/train.log` | Lernkurve Phase 2 (312 Zeilen) |
| `crates/snake-core/assets/alphazero/best.mlp` | **Run 001** bleibt embedded |
