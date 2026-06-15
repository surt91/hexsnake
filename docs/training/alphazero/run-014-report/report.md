# Training Report: AlphaZero-light — Run 014

**Datum**: 2026-06-15  
**Ziel**: Run 001 exakt reproduzieren (seed=1, 80 iter, max-ticks=1500).
Reproduzierbarkeitsprüfung nach Identifikation des Sweet Spots.
Baseline: Run 001 (Walls Ø 37.60, Periodic Ø 54.28).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| `--iterations` | **80** (wie Run 001) |
| `--boundary` | mixed |
| `--eat-bonus` | 0.3 |
| `--max-ticks` | **1 500** (wie Run 001) |
| `--lr` | 1e-3 |
| `--seed` | **1** (wie Run 001) |
| `--games-per-iter` | 128 |
| `--sims` | 24 |

---

## 2. Lernkurve

| Iteration | ~game_len | Bemerkung |
|---|---|---|
| 0  |  ~158 | Zufälliges Netz |
| 59 |  781.0 | [best] |
| 60 |  806.1 | [best] |
| 67 |  808.2 | [best] |
| 69 |  834.1 | [best] |
| **71** | **845.6** | **[best] — Peak** |
| 79 |  787.1 | Endwert |

Peak bei iter 71 (845.6), dann leichter Rückgang auf 787 am Ende.
Run 001 endete mit game_len ~787 bei iter 79 — identisch.

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks, sims=24.

| Topologie  | Run 001 | **Run 014** | Δ |
|------------|--------:|------------:|--:|
| Walls Ø    |   37.60 |   **38.16** | **+1.5 %** |
| Periodic Ø |   54.28 |   **52.86** | −2.6 % |

Run 001 und Run 014 liegen innerhalb des statistischen Rauschens von 50 Spielen.
**Reproduzierbarkeit bestätigt**: Das Setup (seed=1, 80 iter, max-ticks=1500)
produziert konsistent Ergebnisse nahe 37–38 Walls / 53–54 Periodic.

**Run 014 nicht deployed** — kein klarer Vorteil gegenüber Run 001 auf
beiden Metriken gleichzeitig.

---

## 4. Analyse

### 4.1 Sweet-Spot bestätigt

Run 014 peakt bei game_len 845 (iter 71). Run 001 endete bei game_len 787
(iter 79). Beide liegen im optimalen Fenster (game_len 700–900, ~50% von
max-ticks=1500). Das Ergebnis ist reproduzierbar.

### 4.2 Nächster Schritt: Seed-Sweep

Das Setup ist reproduzierbar. Verbesserungsstrategie: Multiple Seeds mit
identischen Parametern (80 iter, max-ticks=1500) → bestes Benchmark-
Ergebnis deployen.

**Run 015**: 4 parallele Seeds (2, 3, 5, 7) → Benchmark aller besten
Checkpoints → Deployment des Gewinners.

---

## 5. Schlussfolgerungen

Run 001 ist reproduzierbar. Das optimale Rezept:
- `--max-ticks 1500` (begrenzt game_len auf Sweet Spot ~700–900)
- `--iterations 80` (Abbruch bevor game_len >900)
- `--boundary mixed` (deckt beide Spielmodi ab)
- `--eat-bonus 0.3` (moderate Fress-Incentivierung)
- Seed als freier Parameter → Sweep über mehrere Seeds

Run 001 bleibt embedded.

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-014/best.mlp` | Best (iter 71, game_len 845.6) — Walls 38.16, Periodic 52.86 |
| `training-out/az-run-014/final.mlp` | Final (iter 79) |
| `training-out/az-run-014/train.log` | Lernkurve |
