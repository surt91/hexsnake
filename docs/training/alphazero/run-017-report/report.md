# Training Report: AlphaZero-light — Run 017

**Datum**: 2026-06-15  
**Ziel**: Testen ob ein größeres Netz (20→128→96→7, 15 751 Parameter vs. 1 639)
die Benchmark-Performance verbessert.
Baseline: Run 016 Seed 15 (Walls Ø 37.26, Periodic Ø 64.96 — 200 Spiele).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| `--iterations` | 150 |
| `--boundary` | mixed |
| `--eat-bonus` | 0.3 |
| `--max-ticks` | 1 500 |
| `--lr` | 1e-3 |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--hidden` | **128 96** (9,6× mehr Parameter als Standard 32 24) |
| `--seed` | 1 / 5 |

---

## 2. Benchmark-Ergebnis (50 Spiele)

| Seed | ckpt | Walls Ø | Periodic Ø | Avg |
|-----:|------|--------:|-----------:|----:|
| Baseline (Run 016 s15, 200 Sp.) | — | 37.26 | 64.96 | 51.11 |
| Run 017 s1 | best | 32.76 | 44.70 | 38.73 |
| Run 017 s1 | final | 32.76 | 44.70 | 38.73 |
| Run 017 s2 | best | 32.68 | 47.74 | 40.21 |
| Run 017 s2 | final | 30.36 | 48.26 | 39.31 |

**Beide Seeds bleiben deutlich unter der Baseline** (avg ~39 vs. 51).

---

## 3. Analyse

### 3.1 Policy-Kollaps / kein Training-Fortschritt

Kennzeichen eines fehlgeschlagenen Lernprozesses:
- best.mlp = final.mlp für Seed 1 (identische Werte) → game_len hat sich nie verbessert
- Sehr kurze Spiele im Benchmark: avg_ticks=455 (Walls) statt erwartet ~700
- Beide Seeds konvergierten zu ähnlichen schwachen Ergebnissen

### 3.2 Diagnose

Das 128-96-Netz (15 751 Parameter) scheitert mit 150 Iterationen:
- **Zu wenige Daten**: 150 × 128 = 19 200 Spiele für 15 751 Parameter ist zu wenig
- **LR zu hoch**: lr=1e-3 kann für ein größeres Netz instabiler sein
- **Sweet-Spot verfehlt**: Das Netz könnte kollabiert sein, bevor es die optimale game_len erreicht

Zum Vergleich: Standard-Netz (1 639 Param) lernt sicher mit 80 × 128 = 10 240 Spielen.
Das 128-96-Netz bräuchte vermutlich 500+ Iterationen oder lr=1e-4.

---

## 4. Schlussfolgerungen

**Negativresultat: Größeres Netz hilft nicht** — zumindest nicht mit diesen Hyperparametern.

Das Standard-Netz (20→32→24→7) bleibt die bessere Wahl für diesen Task:
- Gut angepasst an 20 Features und 6 Aktionen
- Lernt stabil in 80 Iterationen
- Seed-Sweep ist die effektivste Verbesserungsstrategie

**Nächster Schritt (Run 018)**: 
- Warm-Start vom besten Policy (Run 016 Seed 15), lr=3e-4, 100 iter
- Paralleler Seed-Sweep (Seeds 20–22) mit Standard-Architektur

---

## 5. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-017-s1/best.mlp` | Seed 1 (Walls 32.76, Periodic 44.70) — nicht deployed |
| `training-out/az-run-017-s2/best.mlp` | Seed 5 (Walls 32.68, Periodic 47.74) — nicht deployed |
