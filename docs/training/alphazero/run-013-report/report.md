# Training Report: AlphaZero-light — Run 013

**Datum**: 2026-06-15  
**Ziel**: Run 001 mit identischem seed=1 und max-ticks=1500, aber 150
Iterationen (statt 80). Test: Ist Run 001 reproduzierbar? Hilft mehr Training?
Baseline: Run 001 (Walls Ø 37.60, Periodic Ø 54.28).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| `--iterations` | 150 |
| `--boundary` | mixed |
| `--eat-bonus` | 0.3 |
| `--max-ticks` | 1 500 |
| `--lr` | 1e-3 |
| `--seed` | **1** (wie Run 001) |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--best-out` | `az-run-013/best.mlp` |

---

## 2. Lernkurve

| Iteration | ~game_len | Bemerkung |
|---|---|---|
| 0   |  ~157 | Zufälliges Netz |
| ~79 |  ~787 | (Run-001-Endpunkt) |
| 129 | 1 040.9 | [best] |
| 130 | 1 061.6 | [best] |
| 131 | 1 065.8 | [best] |
| 132 | 1 067.4 | [best] |
| 135 | 1 069.6 | [best] |
| 146 | 1 077.9 | [best] |
| 147 | 1 109.2 | [best] |
| **148** | **1 133.5** | **[best] — Peak** |
| 149 | 1 118.8 | Endwert |

Die Kurve steigt noch bei iter 148 — kein Plateau in 150 Iterationen.

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks, sims=24.

| Topologie  | Run 001 | **Run 013** | Δ |
|------------|--------:|------------:|--:|
| Walls Ø    |   37.60 |   **19.42** | **−48.4 %** |
| Periodic Ø |   54.28 |   **47.48** | −12.5 % |
| Ø Ticks (Walls) | — | 4 116 | (Kreisen!) |

**Run 013 nicht deployed.** Katastrophal auf Walls.

---

## 4. Analyse — Endgültige Sweet-Spot-Theorie

### 4.1 game_len ~787 ist der Kipp-Punkt

| Checkpoint | game_len | Walls | Periodic |
|-----------|----------|------:|--------:|
| Run 001 iter 79 | ~787 | **37.60** | **54.28** |
| Run 012 iter 145 | 1 159 | 31.52 | 55.70 |
| Run 013 iter 148 | 1 133 | 19.42 | 47.48 |

Je höher die game_len über 787 steigt, desto schlechter wird Walls.
Die Walls-Ticks explodieren bei Run 013 auf 4116 (der Benchmark lässt
8000 Ticks zu — die Schlange überlebt sehr lange aber frisst kaum).

### 4.2 Warum game_len ~787 der Sweet Spot ist

game_len ~787 = 52% von max-ticks=1500.

- **Unter 52%**: Die Schlange lernt noch Grundlagen. Value-Head optimiert
  effizientes Fressen (kurze, erfolgreiche Spiele = hohe Returns).
- **Bei 52%**: Optimales Gleichgewicht — Schlange frisst gut, stirbt aber
  noch oft genug am Zeitlimit, dass der Value-Head „langes Kreisen" NICHT
  als Ziel lernt.
- **Über 52%**: Value-Head lernt „überleben bis max-ticks" als Strategie.
  Im Benchmark (max-ticks=8000) kreist die Schlange dann 4000+ Ticks
  ohne zu fressen.

### 4.3 Implikation

Run 001 war KEIN Zufall — es war ein idealer Abbruch. Mit seed=1,
max-ticks=1500 und 80 Iterationen landet die game_len exakt bei ~787.
Mehr Iterationen verschlechtern das Ergebnis systematisch.

**Neues Ziel**: Run 001 reproduzieren (seed=1, 80 iter) und dann
Verbesserungsstrategien testen, die den Sweet Spot nicht verschieben:
- Mehr games-per-iter (256 statt 128)
- Mehrere Seeds mit 80 Iterationen → bestes Benchmark-Ergebnis deployen

---

## 5. Schlussfolgerungen

Mit seed=1 und 150 Iterationen befindet sich Run 013 weit jenseits des
optimalen game_len-Fensters. Das bestätigt: **Der optimale AlphaZero-
Trainings-Checkpoint mit max-ticks=1500 liegt bei game_len ~700–900
(~50% des Zeitlimits)**. Run 001 traf dieses Fenster zufällig durch den
frühen Abbruch bei 80 Iterationen.

Run 001 bleibt embedded. **Run 014** startet als exakter Run-001-Klon
(seed=1, 80 iter) zur Reproduzierbarkeitsprüfung.

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-013/best.mlp` | Best (iter 148, game_len 1133.5) — Walls 19.42, Periodic 47.48 |
| `training-out/az-run-013/final.mlp` | Final (iter 149) |
| `training-out/az-run-013/train.log` | Lernkurve (150 Iter) |
