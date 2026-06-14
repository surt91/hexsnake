# Training Report: AlphaZero-light — Run 007

**Datum**: 2026-06-14  
**Ziel**: Erster Run mit implementierter Checkpoint-Speicherung (bester game_len
wird automatisch als `best.mlp` gesichert). Gleiche Hyper-Parameter wie Run 005/006,
neuer Seed 7. Überprüfung ob der beste Checkpoint das Periodic-Ergebnis verbessert.
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
| `--seed` | 7 |
| `--best-out` | `training-out/az-run-007/best.mlp` (**neu**) |

**Neu**: `train_alphazero.py` speichert jetzt automatisch das Modell mit dem
höchsten `avg_game_len` als `best.mlp`. Der Log-Eintrag enthält `[best]` bei
jedem neuen Maximum.

---

## 2. Lernkurve

| Iteration | Policy-Loss | ~game_len | Bemerkung |
|---|---|---|---|
| 0   | — |   183.2 | |
| 50  | — |   900.5 | |
| 100 | — | 1 074.6 | |
| 137 | — | **1 998.1** | **[best] — Globaler Peak** |
| 150 | — |   916.6 | Rückgang |
| 200 | — | 1 250.0 | |
| 300 | — | 1 216.0 | Plateau |
| 400 | — | 1 402.8 | |
| 450 | — | 1 544.8 | |
| 500 | — | 1 512.8 | |
| 550 | — | 1 321.7 | |
| 599 | — | 1 220.6 | Endwert (exportiert als `final.mlp`) |

**Beobachtung**: Der Peak liegt bei iter 137 (23 % der Iterationen) — viel früher
als in Run 005/006 (iter 223/426). Die Kurve oszilliert danach ohne zweiten Peak
und endet bei game_len=1220 (38 % unterhalb des Peaks). Die Checkpoint-Speicherung
funktioniert wie erwartet.

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks, sims=24. **Benchmarked: best.mlp
(iter 137, game_len 1998.1)**

| Topologie  | Run 001 | Run 005 | Run 006 | **Run 007 (best)** | Δ vs. 001 |
|------------|--------:|--------:|--------:|-------------------:|----------:|
| Walls Ø    |   37.60 |   24.10 |   27.94 |           **30.16** |  −19.8 % |
| Periodic Ø |   54.28 |   56.04 |   50.88 |           **59.18** | **+9.0 %** |

**Periodic: Bestes Ergebnis aller AZ-Runs.** Run-007 schlägt run-001 auf Periodic
klar (+9.0 %). Auf Walls bleibt eine Regression (−19.8 %).

---

## 4. Analyse

### 4.1 Checkpoint-Speicherung bestätigt

Der Unterschied zwischen Endwert (game_len=1220, entspricht ~25 Äpfeln) und dem
gespeicherten best.mlp (game_len=1998 → 30.16 Walls, 59.18 Periodic) zeigt den
Wert der Checkpoint-Speicherung. Ohne sie wäre run-007 ähnlich schlecht wie
run-006 (der Endwert war ~12 % schlechter).

### 4.2 Walls vs. Periodic

Das persistente Muster über alle max-ticks=4000-Runs (005–007):
- **Periodic verbessert sich**: 56.04 → 50.88 → 59.18
- **Walls verschlechtert sich**: 24.10 → 27.94 → 30.16 (verbessernd, aber unter 37.60)

**Hypothese**: Mixed-Boundary-Training mit max-ticks=4000 exponiert das Netz
überwiegend an Situationen, die langen Torus-Spielen ähneln (Schlange lebt lang,
Ränder sind passierbar). Walls-spezifische Überlebenstaktiken (Umkehren in
Wandnähe) werden weniger trainiert. Run 001 mit max-ticks=1500 und nur 300
Iterationen hatte zufällig die richtige Mischung für Walls.

### 4.3 Peak-Timing variiert stark

| Run | Peak-Iter | Max-Iter | Peak/Max |
|-----|----------:|----------:|---------:|
| 005 | 223 | 300 | 74 % |
| 006 | 426 | 600 | 71 % |
| 007 | **137** | **600** | **23 %** |

Seed 7 führt zu einem frühen Peak ohne weiteren Anstieg. Die Oscillation ist
stark seed-abhängig. Das macht frühe Iteration-Abbrüche (Early Stopping) schwierig.

---

## 5. Schlussfolgerungen

**Run-007 bleibt nicht deployed** (Walls −19.8 %). Run 001 bleibt eingebettet.

Run-007 zeigt jedoch: Das Periodic-Problem ist lösbar (59.18 vs. 54.28).
Das Walls-Problem liegt in der Trainingsdomäne.

**Mögliche nächste Schritte:**

1. **Walls-only Training**: AZ auf `--boundary walls` trainieren, dann auf Periodic
   evaluieren. Vermutlich umgekehrtes Muster (gutes Walls, schlechteres Periodic).
2. **Höhere LR im Früh-Training, LR-Decay danach**: Peak kommt früh (iter 100–230)
   → danach LR-Decay könnte das Peak-Niveau konsolidieren statt dass die Policy
   überschreibt.
3. **Kleineres Modell**: Weniger Überanpassen an Training-Seeds → bessere
   Generalisierung? Run 001 nutzte 20→32→24→7, spätere Runs gleiche Architektur.
4. **Early Stopping nach N schlechten Iterationen**: Z.B. stopp wenn 50
   aufeinanderfolgende Iter. keinen neuen best-checkpoint setzen.

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-007/best.mlp` | Bester Checkpoint (iter 137, game_len 1998.1) |
| `training-out/az-run-007/final.mlp` | Endmodell (iter 599, game_len 1220.6) |
| `training-out/az-run-007/train.log` | Lernkurve (600 iter) |
| `crates/snake-core/assets/alphazero/best.mlp` | **Run 001** bleibt embedded |
