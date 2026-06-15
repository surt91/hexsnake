# Training Report: AlphaZero-light — Run 017

**Datum**: 2026-06-15  
**Ziel**: Testen ob ein größeres Netz (20→128→96→7, 15 751 Parameter vs. 1 639)
die Benchmark-Performance verbessert.
Baseline: Run 015 Seed 5 (Walls Ø 36.66, Periodic Ø 60.17 — 200-Spiele-Wert).

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
| `--hidden` | **128 96** (9,6× mehr Parameter als Standard) |
| `--seed` | 1 / 5 |

Änderung gegenüber Run 016: `--hidden 128 96` statt `32 24`.
Benötigte Erweiterungen (alle in diesem Commit deployed):
- `az_net.py`: `AZNet` mit konfigurierbaren Hidden-Layers
- `train_alphazero.py`: `--hidden`-Argument, Dims-Übergabe an `az_selfplay`
- `snake-py/src/lib.rs`: optionaler `dims`-Parameter für `az_selfplay`

---

## 2. Ergebnisse

*Werden nach Training-Abschluss ergänzt.*

---

## 3. Analyse

*Wird nach Training-Abschluss ergänzt.*

---

## 4. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-017-s1/best.mlp` | Seed 1 bestes Checkpoint |
| `training-out/az-run-017-s1/final.mlp` | Seed 1 finales Checkpoint |
| `training-out/az-run-017-s2/best.mlp` | Seed 5 bestes Checkpoint |
| `training-out/az-run-017-s2/final.mlp` | Seed 5 finales Checkpoint |
