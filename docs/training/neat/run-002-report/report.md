# Training Report: NEAT — Run 002

**Datum**: 2026-06-14  
**Ziel**: Mehr Budget (500 Gen, Pop=200) und stärkerer Score-Druck (fitness×1000).
Baseline: Run 001 (Walls Ø 38.92, Periodic Ø 56.84).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | NEAT (Rust, `snake-train neat`) |
| `--generations` | 500 |
| `--population` | 200 |
| `--games` | 10 |
| `--max-ticks` | 2 000 |
| `--boundary` | mixed (via `--mixed`) |
| `--seed` | 2 |

**Fitness-Änderung gegenüber Run 001:**

| Formel | Run 001 | Run 002 |
|---|---|---|
| Fitness | `score×100 + ticks×0.1` | `score×1000 + ticks×0.1` |

---

## 2. Lernkurve

| Generation | Best Fitness | Mean Fitness | Spezies | Nodes |
|---|---|---|---|---|
| 0   | 1 145.7 |  241.8 | 1 | 27 |
| 100 | ~3 500  | ~1 200 | — | — |
| 250 | ~4 200  | ~1 800 | — | — |
| 400 | ~4 500  | ~2 300 | — | — |
| 496 | 4 497.7 | 2 611.2 | 1 | 45 |
| 499 | 5 012.7 | 2 764.1 | 1 | 46 |
| **Best** | **5 512.1** | — | 1 | ~46 |

Mit `fitness = score × 1000`, entspricht Best 5512 einem Score von
~5.5 Äpfeln/Spiel. Zum Vergleich: Run-001-Best ergäbe mit ×1000 umgerechnet
~3800 (d. h. ~3.8 Äpfel). Run 002 isst mehr, aber der Benchmark zeigt
schlechteres Gesamtergebnis.

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks.

| Topologie  | Run 001 | **Run 002** | Δ |
|------------|--------:|------------:|--:|
| Walls Ø    |   38.92 |   **28.78** | −26.1 % |
| Periodic Ø |   56.84 |   **49.94** | −12.1 % |

**Regression auf beiden Topologien.** Run 001 bleibt deployed.

---

## 4. Analyse

Die fitness×1000-Änderung hat NEAT in eine lokale Optimumfalle geführt:

- **Kurze aggressive Spiele**: Mit Score dominierend gegenüber Ticks
  selegiert NEAT Strategien, die wenige Äpfel schnell fressen und
  dann sterben. Diese generalisieren schlecht auf längere Spiele
  (8 000-Tick-Benchmark).

- **Ticks-Bonus als implizites Robustheits-Signal**: Die alte Formel
  `score×100 + ticks×0.1` belohnte implizit auch robuste Überlebens-
  strategien. Wird dieser Bonus klein, verliert NEAT den Selektionsdruck
  für Strategien, die auch mit langer Schlange sicher navigieren.

- **Ein Spezies**: Durch die geänderte Fitness konvergiert NEAT auf
  eine einzige Spezies (keine Diversifizierung). Run 001 hatte ebenfalls
  eine Spezies, aber Run 002 entwickelt keine Nischenstrategien.

**Fitness-Formel zurückgesetzt** auf `score×100 + ticks×0.1` nach
diesem Experiment.

---

## 5. Schlussfolgerungen

- `score×1000` schadet NEAT — die bestehende Formel ist besser kalibriert
- Für mehr Fressdruck bei NEAT: Distanz-Shaping als Fitness-Komponente
  hinzufügen (analog zu `REWARD_APPROACH` bei DQN/PPO), statt Score-Gewicht
- Mehr Budget (500 Gen × 200 Pop) bringt per se keinen Vorteil wenn die
  Fitnessfunktion falsch ausgerichtet ist

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/neat-run-002/best.neat` | Gewichte Run 002 (nicht deployed) |
| `training-out/neat-run-002/train.log` | Fitness-Log (500 Generationen) |
| `crates/snake-core/assets/neat/best.neat` | **Run 001** bleibt embedded |
