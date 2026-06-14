# Training Report: MLP-GA — Run 007

**Datum**: 2026-06-14  
**Ziel**: Stärkerer Score-Druck (fitness×1000) bei mittlerem Budget (500 Gen, Pop=200).
Baseline: Run 005 (Walls Ø 91.40, Periodic Ø 125.12) — bestes bisheriges Ergebnis.

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | MLP-GA (Rust, `snake-train`) |
| Architektur | 20→32→24→6 |
| `--generations` | 500 |
| `--population` | 200 |
| `--games` | 10 |
| `--max-ticks` | 2 000 |
| `--boundary` | mixed (via `--mixed`) |
| `--seed` | 6 |
| Training-Out | `training-out/mlp-ga-run-006/` |

**Fitness-Änderung:**

| Formel | Run 005 | Run 007 |
|---|---|---|
| Fitness | `score×100 + ticks×0.1` | `score×1000 + ticks×0.1` |

---

## 2. Lernkurve

| Generation | Best Fitness | Mean Fitness |
|---|---|---|
| 0   | 5 463.9 |   544.6 |
| 1   | 10 202.9 | 1 325.9 |
| 2   | 13 243.9 | 2 648.9 |
| 496 | 70 985.3 | 51 281.0 |
| 499 | 72 857.0 | 54 584.9 |
| **Best** | **76 275.8** | — |

Mit `fitness = score × 1000` entspricht Best 76 275 ≈ 76 Äpfeln/Spiel.
Mit der alten ×100-Formel würde diese Fitness ~763 ergeben — weit unter
Run-005-Niveau (12 434 alt, entspricht ~124 Äpfeln).

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks.

| Topologie  | Run 005 | **Run 007** | Δ |
|------------|--------:|------------:|--:|
| Walls Ø    |   91.40 |   **57.48** | −37.1 % |
| Periodic Ø |  125.12 |   **74.64** | −40.4 % |

**Schwere Regression.** Run 005 bleibt deployed.

---

## 4. Analyse

`fitness×1000` ändert die Selektionslandschaft grundlegend:

- **Kurze aggressive Spiele dominieren**: GA evolviert Genome, die in
  2 000-Tick-Spielen möglichst viel fressen und dann sterben. Im
  8 000-Tick-Benchmark sterben diese Strategien früh.

- **Ticks-Bonus als Robustheits-Signal**: `ticks×0.1` belohnte implizit
  Überlebensstrategien — essentiell für späte Spielsituationen. Mit
  `score×1000` ist dieser Druck 10 000× kleiner geworden und vernachlässigbar.

- **Trainings-Benchmark-Gap**: Training mit max-ticks=2 000 begrenzt, was
  die Genome sehen. Benchmark mit max-ticks=8 000 deckt komplexere Situationen
  auf, die im Training nicht vorkamen.

**Konsequenz**: `fitness = score×100 + ticks×0.1` (Run-005-Formel) nach
diesem Experiment wiederhergestellt.

---

## 5. Schlussfolgerungen

- `score×1000` schadet MLP-GA erheblich
- 500 Gen/Pop=200 reicht nicht für MLP-GA (Run 005 brauchte ~5 000 Gen/Pop=512)
- Fitness-Tuning ist nicht der Weg zu besseren MLP-GA-Ergebnissen
- Für Verbesserungen über Run 005: mehr Budget (> 5 000 Gen) oder CMA-ES

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/mlp-ga-run-006/best.mlp` | Gewichte Run 007 (nicht deployed) |
| `training-out/mlp-ga-run-006/train.log` | Fitness-Log (500 Generationen) |
| `crates/snake-core/assets/mlp-ga/best.mlp` | **Run 005** bleibt embedded |
