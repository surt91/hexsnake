# Training Report: AlphaZero-light — Run 003

**Datum**: 2026-06-14  
**Ziel**: MCTS eat-Bonus 0.5 → 1.0 — stärkte im Test die Schlangenlänge.
Baseline: Run 001 (Walls Ø 37.60, Periodic Ø 54.28).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | AlphaZero-light Gradient (Self-Play in Rust + PyTorch-Update) |
| `--iterations` | 200 |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--temperature` | 1.0 |
| `--epochs` | 4 |
| `--boundary` | mixed (50/50 Walls/Torus) |
| `--max-ticks` | 1 500 |
| `--seed` | 3 |
| Architektur | 20→32→24→7 (6 Policy-Logits + 1 Value-Kopf) |
| GPU | CUDA (nvidia) |

**Reward vs. Run 001:**

| Größe | Run 001 | Run 003 |
|---|---|---|
| MCTS Fress-Bonus | 0.3 | **1.0** |
| Self-Play Fressen | +1.0 | +3.0 |
| Self-Play Lebenskosten | −0.005/Tick | −0.01/Tick |

---

## 2. Lernkurve

| Iteration | Policy-Loss | ~game_len | Bemerkung |
|---|---|---|---|
| 0   | 1.588 |  182 | Zufälliges Netz |
| 18  | 0.087 |  495 | |
| 38  | 0.060 |  516 | |
| 58  | 0.043 |  824 | |
| 78  | 0.005 |  964 | |
| 98  | 0.001 | 1 134 | Policy konvergiert |
| 118 | 0.011 | 1 274 | |
| 158 | 0.002 | **1 474** | Nahe Zeitlimit |
| 199 | 0.000 | **1 477** | Stabil bei max-ticks |

Die mittlere Spiellänge stieg stetig von 182 auf **1 477** — fast das
gesamte max-ticks-Limit von 1 500. Das MCTS mit eat-Bonus=1.0 läuft
(wie erwartet) sehr lang und komplexe Board-Zustände entstehen.

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks, sims=24.

| Topologie  | Run 001 | **Run 003** | Δ |
|------------|--------:|------------:|--:|
| Walls Ø    |   37.60 |   **22.58** | −40.0 % |
| Periodic Ø |   54.28 |   **47.70** | −12.1 % |

**Starke Regression** trotz hervorragender Trainingsperformanz.
Run 001 bleibt deployed.

---

## 4. Analyse: Training-Distribution-Mismatch

Das Paradoxon: game_len im Training nahe 1 500 → Score im Benchmark 22.

Ursache: **Die Policy sieht im Training nie lange Schlangen.**

Mit max-ticks=1 500 und game_len=1 477 endet jede Partie am Zeitlimit,
bevor die Schlange viele Felder belegt. Die MCTS-Suche führt mit
eat-Bonus=1.0 zu agressivem Fressen — aber die *destillierte Policy*
(die was das Netz selbst gelernt hat) generalisiert nicht auf die
komplexen Brettzustände mit langer Schlange, die im 8 000-Tick-Benchmark
entstehen. Jenseits der Trainings-Tickgrenze ist die Policy
out-of-distribution.

Konkret:
- Training: Schlange frisst ~15–30 Äpfel in 1 500 Ticks (est.), Länge ≤ 35
- Benchmark: Schlange stirbt bei Ticks ~2 240 mit Score ~22
  → Zustände mit Schlangenlänge > 35 wurden nie gelernt

**MCTS kompensiert nicht**: Selbst mit sims=24 kann die Suche schlechte
Wertschätzungen der Policy bei langen Schlangen nicht korrigieren —
der Value-Head ist nicht kalibriert für diese Zustände.

---

## 5. Schlussfolgerungen

- eat=1.0 im MCTS ist **gut** als Trainings-Signal: die Schlange lernt
  aktiv Futter zu suchen. Das Problem ist die Training-Distribution.
- **Lösung**: `--max-ticks` deutlich erhöhen (4 000–8 000), damit das
  Netz auch lange Schlangen-Situationen lernt — auf Kosten der
  Trainingsgeschwindigkeit (längere Spiele = mehr Clones in MCTS).
- Alternativ: `--sims` erhöhen zur Inferenz (64+), damit MCTS die
  Policy-Schwäche bei langen Schlangen ausgleicht.

---

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-003/best.mlp` | Gewichte Run 003 (nicht deployed) |
| `training-out/az-run-003/train.log` | Lernkurve (200 iter) |
| `crates/snake-core/assets/alphazero/best.mlp` | **Run 001** bleibt embedded |
