# Training Report: AlphaZero-light — Run 020

**Datum**: 2026-06-15  
**Ziel**: Testen ob `--games-per-iter 256` (doppelte Datenmenge) balanced-Policies fördert.
Baseline: Run 016 Seed 15 (Walls 37.26, Periodic 64.96, Avg 51.11 — 200 Spiele).

---

## 1. Setup (alle 8 Seeds identisch außer `--seed`)

| Parameter | Wert |
|---|---|
| `--iterations` | 80 |
| `--boundary` | mixed |
| `--eat-bonus` | 0.3 |
| `--max-ticks` | 1 500 |
| `--lr` | 1e-3 |
| `--games-per-iter` | **256** (doppelt vs. vorher 128) |
| `--sims` | 24 |
| `--hidden` | 32 24 (Standard) |
| `--seed` | 40–47 |

Hypothese: Mehr Spiele pro Iteration → robustere Gradienten → balanced statt spezialisiert.

---

## 2. Ergebnisse

*Werden nach Training-Abschluss ergänzt.*

---

## 3. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-020-s{40..47}/best.mlp` | Beste Checkpoints je Seed |
| `training-out/az-run-020-s{40..47}/final.mlp` | Finale Checkpoints je Seed |
