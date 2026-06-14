# Training Report: AlphaZero-light — Run 008

**Datum**: 2026-06-14  
**Ziel**: Zweiphasiges Training: erst walls-only (Phase 1), dann mixed Fine-Tuning
mit Warm-Start vom Phase-1-Best (Phase 2). Hypothese: Walls-Training erzwingt
robuste Navigationstaktik; mixed Fine-Tuning fügt Periodic-Fähigkeiten hinzu.
Baseline: Run 001 (Walls Ø 37.60, Periodic Ø 54.28).

---

## 1. Setup

### Phase 1 — Walls-only (1 000 Iterationen)

| Parameter | Wert |
|---|---|
| `--iterations` | 1 000 |
| `--boundary` | **walls** |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--max-ticks` | 4 000 |
| `--seed` | 8 |
| `--best-out` | `az-run-008-p1/best.mlp` |

### Phase 2 — Mixed Fine-Tuning (500 Iterationen)

| Parameter | Wert |
|---|---|
| `--iterations` | 500 |
| `--boundary` | **mixed** |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--max-ticks` | 4 000 |
| `--seed` | 8 |
| `--load` | `az-run-008-p1/best.mlp` (**Warm-Start**) |
| `--best-out` | `az-run-008-p2/best.mlp` |

Gesamt: 1 500 Iterationen (5× Run 001).

---

## 2. Lernkurve

### Phase 1 (walls-only)

| Iteration | ~game_len | Bemerkung |
|---|---|---|
| 0   |   119.0 | Zufälliges Netz |
| 100 |   888.9 | Schnelles Lernen |
| **193** | **1 386.4** | **[best] — Peak** |
| 300 | 1 168.7 | Rückgang auf Plateau |
| 500 | 1 168.1 | Plateau stabil |
| 700 | 1 170.9 | keine weitere Verbesserung |
| 999 | 1 169.7 | Endwert |

Phase 1 konvergiert früh bei iter 193 (game_len 1386) auf ein stabiles Plateau
bei ~1168 (iter 300–999). 800 weitere Iterationen bringen keine Verbesserung.

### Phase 2 (mixed Fine-Tuning)

| Iteration | ~game_len | Bemerkung |
|---|---|---|
| 0   | 1 635.6 | Warm-Start von P1 Best |
| 25  |   914.3 | Sofortiger Einbruch |
| 100 | 1 299.1 | Erholung |
| **153** | **2 320.1** | **[best] — Peak** |
| 200 | 1 358.9 | Rückgang |
| 300 | 1 479.3 | Zweite Stabilisierung |
| 499 | 1 540.5 | Endwert |

Phase 2 startet hoch (1635 dank Warm-Start), bricht dann sofort ein (Buffer wird
mit neuen mixed-Daten gefüllt), erholt sich auf einen Peak bei iter 153 (2320),
der aber unter den Peaks von Run 005–007 (2529–2779) liegt.

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks, sims=24.

| Topologie  | Run 001 | P1 best (iter 193) | **P2 best (iter 153)** | Δ vs. 001 |
|------------|--------:|-------------------:|-----------------------:|----------:|
| Walls Ø    |   37.60 |              26.44 |                **28.30** | −24.7 % |
| Periodic Ø |   54.28 |              20.98 |                **12.88** | **−76.3 %** |
| Ø Ticks (Walls) | — | 1 310 | 900 | — |
| Ø Ticks (Periodic) | — | 5 897 | 7 137 | — |

**Schwere Regression.** Run 001 bleibt deployed.

---

## 4. Analyse: Sichere-Kreisen-Lokaloptimum

### 4.1 Das „Sichere Kreisen"-Problem

Die extremen avg_ticks-Werte auf Periodic (5 897 / 7 137 Ticks bei nur 21/13
Score) zeigen: Die Schlange **überlebt sehr lange, isst aber kaum**. Sie kreist
sicher auf Periodic — der Value Head bewertet Überleben sehr hoch, Futter kaum.

Ursache: Walls-only-Training (Phase 1) belohnt vor allem **Nicht-Sterben**.
Auf einem Walls-Feld ist der Haupttodgrund die Wand → die Policy lernt,
Wände zu meiden. Mit max-ticks=4000 und game_len ~1168 (Plateau) überschätzt
der Value Head den Wert des Überlebens gegenüber dem Fressen.

### 4.2 Buffer-Initialisierungsproblem in Phase 2

Phase 2 startet bei iter 25 mit game_len 914 — deutlich schlechter als der
Warm-Start-Wert (1635). Ursache: Der Replay-Buffer füllt sich mit neuen
mixed-Daten, die der alten Walls-Policy fremd sind. Die Policy muss sich erst
an das neue Boundary-Regime anpassen. Der Peak bei iter 153 (2320) ist dann
ein schnelles Anpassen an mixed — aber der zuvor eingebrannte „sichere Kreisen"-
Bias im Value Head bleibt.

### 4.3 Vergleich mit direktem Mixed-Training

| Run | Phase | Boundary | Peak game_len | Walls | Periodic |
|-----|-------|---------|------:|------:|------:|
| 007 | — | mixed | 1 998 (iter 137) | 30.16 | **59.18** |
| 008 P1 | 1 | walls | 1 386 (iter 193) | 26.44 | 20.98 |
| 008 P2 | 2 | mixed | 2 320 (iter 153) | 28.30 | 12.88 |

Run 007 (mixed, kein Warm-Start) ist auf Periodic fast 5× besser als Run 008 P2,
obwohl Run 008 P2 einen höheren Peak in game_len erzielt. Walls-Vortraining
schadet dem Periodic-Ergebnis dramatisch.

---

## 5. Schlussfolgerungen

**Zweiphasiger Ansatz (walls → mixed) funktioniert nicht.**

- Phase 1 konvergiert schnell auf ein Plateau; 800 Extra-Iterationen sind verschwendet.
- Walls-only Training baut einen Value Head, der Überleben über Fressen priorisiert.
- Das Mixed Fine-Tuning in Phase 2 kann diesen Bias nicht in 500 Iterationen überwinden.
- Das Periodic-Ergebnis ist dramatisch schlechter als bei purem Mixed-Training.

**Was hätte funktionieren können:**
- Phase 1 deutlich kürzer (100–150 Iter statt 1000) — nur bis zum echten Peak,
  bevor das Plateau einsetzt.
- Erhöhter eat-Bonus in Phase 1, um Fress-Anreize trotz Walls-Fokus zu erhalten.
- Phase 2 länger (1000+ Iter) für stärkere Überschreibung des Value Head.

Momentan bietet direktes Mixed-Training (Run 007, kein Warm-Start) bessere
Periodic-Ergebnisse. Der Walls-Rückstand gegenüber Run 001 bleibt ungelöst.

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-008-p1/best.mlp` | Phase 1 Best (iter 193, game_len 1386.4) |
| `training-out/az-run-008-p1/final.mlp` | Phase 1 Final (iter 999, game_len 1169.7) |
| `training-out/az-run-008-p1/train.log` | Lernkurve Phase 1 (1000 iter) |
| `training-out/az-run-008-p2/best.mlp` | Phase 2 Best (iter 153, game_len 2320.1) |
| `training-out/az-run-008-p2/final.mlp` | Phase 2 Final (iter 499, game_len 1540.5) |
| `training-out/az-run-008-p2/train.log` | Lernkurve Phase 2 (500 iter) |
| `crates/snake-core/assets/alphazero/best.mlp` | **Run 001** bleibt embedded |
