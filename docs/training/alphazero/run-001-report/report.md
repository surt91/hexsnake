# Training Report: AlphaZero-light — Run 001

**Datum**: 2026-06-14  
**Ziel**: Erster produktiver AlphaZero-light-Lauf (Gradient-Training).
Baseline bisher: Smoke-Run (Walls Ø 30, Periodic Ø 54).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | AlphaZero-light Gradient (Self-Play in Rust + PyTorch-Update) |
| `--iterations` | 80 |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--temperature` | 1.0 |
| `--epochs` | 4 |
| `--boundary` | mixed (50/50 Walls/Torus) |
| `--max-ticks` | 1 500 |
| `--seed` | 1 |
| Architektur | 20→32→24→7 (6 Policy-Logits + 1 Value-Kopf) |
| GPU | CUDA (nvidia) |

**Besonderheit**: Self-Play läuft komplett in Rust (GIL freigegeben → alle Cores).
Python macht nur den Gradienten-Schritt. Export-Self-Check (numpy/Torch == Rust):
`err 1.49e-08` — Gewichtsformat korrekt.

**Bug-Fix während Training**: `export_mlp()` erstellte das Ausgabeverzeichnis nicht
(FileNotFoundError beim ersten Durchlauf). Fix: `os.makedirs(dir, exist_ok=True)`
in `export.py`. Commit `956bfd4`.

---

## 2. Lernkurve

| Iteration | Policy-Loss | Value-Loss | ~game_len | Bemerkung |
|---|---|---|---|---|
| 0   | 1.582 | 0.239 | 150  | Zufälliges Netz, kurze Spiele |
| 20  | 0.134 | 0.037 | 522  | Policy schnell konvergiert |
| 40  | 0.018 | 0.104 | 617  | Policy fast saturiert |
| 60  | 0.032 | 0.086 | 806  | Spiellänge weiter steigend |
| 79  | 0.034 | 0.062 | 787  | Stabil; game_len ~780–850 |

Policy-Loss fällt von 1.58 auf ~0.03 in nur 20 Iterationen. Value-Loss
bleibt kleiner (0.04–0.14) und ist rauschiger. Die mittlere Spiellänge
stieg von 150 (iter 0) auf ~800 (iter 40+) bei max. 1500 Ticks.

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks.

| Topologie  | Smoke-Run | **Run 001** | Δ |
|------------|----------:|------------:|--:|
| Walls Ø    |     30.04 |   **37.60** | +25.2 % |
| Periodic Ø |     53.76 |   **54.28** | +1.0 % |

Klare Verbesserung auf Walls; Periodic kaum verändert.

---

## 4. Vergleich mit anderen Methoden

| Methode       | Walls Ø | Periodic Ø |
|---------------|--------:|-----------:|
| MLP-GA 005    |   91.40 |     125.12 |
| NEAT run-001  |   38.92 |      56.84 |
| **AZ run-001**| **37.60** | **54.28** |
| PPO run-001   |   38.24 |      58.62 |
| DQN run-001   |   22.96 |      48.18 |

AlphaZero und NEAT auf vergleichbarem Niveau. Beide deutlich unter MLP-GA.

---

## 5. Beobachtungen

- **Policy convergiert sehr schnell**: Von 1.58 auf ~0.03 Policy-Loss in
  nur 20 Iterationen. Das Netz lernt MCTS-Besuchsverteilungen nachzuahmen,
  aber ob das in besseres Spiel mündet, hängt von der Qualität der MCTS ab.

- **game_len hoch, Score moderat**: ~800 Ticks Überlebenszeit aber nur 37
  Äpfel auf Walls bedeutet ineffizientes Fressen. Das Netz kreist sicher
  statt aktiv anzujagen — typisch wenn das Suchbudget (sims=24) die
  Planungshorizont-Tiefe begrenzt.

- **MCTS-Budget ist der Schlüssel**: Mit `--sims 48+` zur Inferenz wächst
  die Spielstärke, aber `embedded()` spielt mit demselben Budget wie das
  Training — muss bei Änderungen angepasst werden.

- **GPU für PyTorch, CPU für Self-Play**: CUDA für Gradientenschritt; die
  Rust-Self-Play-Loops laufen auf CPU (rayon). Gut skalierend.

---

## 6. Fazit

Run 001 ist ein vollständiger Proof-of-Concept. Das Gradient-Training funktioniert
end-to-end, und der Export-Self-Check (err=1.5e-8) bestätigt das Gewichtsformat.

Für stärkere Ergebnisse:
- Mehr Iterationen (200–500)
- Höheres Suchbudget (`--sims 48+`) zur Trainingszeit und Inferenz
- Mehr Spiele pro Iteration (256+)
- Eventuell Reward-Anpassung: explizitere Belohnung für Futter statt nur Überleben

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/alphazero/best.mlp` | Eingecheckt (Run 001) |
| `training-out/az-run-001/best.mlp` | Exportiertes Netz |
| `training-out/az-run-001/train.log` | Lernkurve |
