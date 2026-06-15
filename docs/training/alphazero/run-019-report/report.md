# Training Report: AlphaZero-light — Run 019

**Datum**: 2026-06-15  
**Ziel**: Breiterer Seed-Sweep (Seeds 30–38) um die Baseline zu übertreffen.
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
| `--seed` | 30–38 |

---

## 2. Ergebnisse

*Werden nach Training-Abschluss ergänzt.*

---

## 3. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-019-s{30..38}/best.mlp` | Beste Checkpoints je Seed |
| `training-out/az-run-019-s{30..38}/final.mlp` | Finale Checkpoints je Seed |
