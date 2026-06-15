# Training Report: AlphaZero-light — Run 021

**Datum**: 2026-06-15  
**Ziel**: Letzter Seed-Sweep (Seeds 60–68) um die Baseline zu übertreffen.
Baseline: Run 016 Seed 15 (Walls 37.26, Periodic 64.96, Avg 51.11 — 200 Spiele).

---

## 1. Setup (alle 9 Seeds identisch außer `--seed`)

| Parameter | Wert |
|---|---|
| `--iterations` | 80 |
| `--boundary` | mixed |
| `--eat-bonus` | 0.3 |
| `--max-ticks` | 1 500 |
| `--lr` | 1e-3 |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--hidden` | 32 24 (Standard) |
| `--seed` | 60–68 |

Erkenntnis aus Run 020: games-per-iter=256 ist schlechter als 128. Zurück zur
bewährten Konfiguration.

---

## 2. Benchmark-Ergebnis (50 Spiele, best.mlp / final.mlp)

| Seed | Walls best | P best | Avg best | Walls final | P final | Avg final |
|-----:|-----------:|-------:|---------:|------------:|--------:|----------:|
| Baseline (Run 016 s15) | 37.26 | 64.96 | 51.11 | — | — | — |
| s60 | 35.18 | 56.36 | 45.77 | 37.80 | 58.46 | 48.13 |
| s61 | 26.66 | 55.14 | 40.90 | 27.36 | 58.58 | 42.97 |
| s62 | 31.24 | 47.98 | 39.61 | 31.24 | 47.98 | 39.61 |
| s63 | 30.06 | 49.12 | 39.59 | 23.28 | 50.70 | 36.99 |
| s64 | 28.74 | 52.58 | 40.66 | 28.74 | 52.58 | 40.66 |
| s65 | 37.68 | 51.14 | 44.41 | 39.00 | 50.02 | 44.51 |
| s66 | 31.80 | 55.30 | 43.55 | 30.80 | 56.84 | 43.82 |
| s67 | 31.20 | 69.36 | 50.28 | 29.68 | 67.38 | 48.53 |
| **s68** | **40.04** | **67.40** | **53.72** | **38.44** | **66.18** | **52.31** |

### Seed 68 Verifikation (200 Spiele)

| Topologie | Run 016 s15 (Baseline, 200 Sp.) | **Run 021 s68 (200 Sp.)** | Δ |
|-----------|--------------------------------:|-------------------------:|--:|
| Walls Ø | 37.26 | **40.72** | **+9.3 %** |
| Periodic Ø | 64.96 | **75.74** | **+16.6 %** |
| Avg Ø | 51.11 | **58.23** | **+13.9 %** |

**Bestes je gemessenes Ergebnis.** Seed 68 schlägt die Baseline auf BEIDEN Metriken
mit zweistelligen Prozentwerten. Sofort deployed.

---

## 3. Analyse

### 3.1 Seed 68 — eine Ausnahme

Von 9 Seeds in diesem Run dominiert nur Seed 68 klar. Die anderen 8 Seeds liegen
unter oder nahe der Baseline. Dies bestätigt das Grundmuster: Die Policy-Qualität
hängt stark vom Seed ab, und „gute" Seeds sind selten aber deutlich erkennbar.

### 3.2 Warum Seed 68?

Ohne Einblick in die genaue Trainingsgeschichte (Logs buffered) lässt sich nur sagen:
Seed 68 fand einen Lernpfad, der BEIDE Topologien gleichzeitig verbessert hat.
Der best.mlp-Checkpoint (peak self-play game_len) entspricht hier auch dem besten
Benchmark-Checkpoint — seltenes Glück, das bei vielen Runs nicht eintritt.

### 3.3 Projektergebnis über alle Runs

| Run | Deployed | Walls (200 Sp.) | Periodic (200 Sp.) | Avg |
|-----|----------|----------------:|-------------------:|----:|
| Run 001 | ja | 35.40 | 51.88 | 43.64 |
| Run 015 s5 | ja | 36.66 | 60.17 | 48.42 |
| Run 016 s15 | ja | 37.26 | 64.96 | 51.11 |
| **Run 021 s68** | **ja** | **40.72** | **75.74** | **58.23** |

Verbesserung gegenüber dem ursprünglichen Run 001: Walls +15%, Periodic +46%, Avg +33%.

---

## 4. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-021-s68/best.mlp` | **Seed 68 (Walls 40.72, Periodic 75.74) — deployed!** |
| `training-out/az-run-021-s{60..68}/best.mlp` | Alle Checkpoints |
| `crates/snake-core/assets/alphazero/best.mlp` | **Run 021 Seed 68 — deployed** |
