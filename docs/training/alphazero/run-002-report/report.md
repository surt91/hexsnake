# Training Report: AlphaZero-light — Run 002

**Datum**: 2026-06-14  
**Ziel**: Mehr Budget (2 000 Iterationen, 6 h), angepasster Reward (Futter-Priorität).
Baseline: Run 001 (Walls Ø 37.60, Periodic Ø 54.28).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | AlphaZero-light Gradient (Self-Play in Rust + PyTorch-Update) |
| `--iterations` | 2 000 |
| `--games-per-iter` | 256 |
| `--sims` | 32 |
| `--temperature` | 1.0 |
| `--epochs` | 4 |
| `--boundary` | mixed (50/50 Walls/Torus) |
| `--max-ticks` | 800 |
| `--seed` | 2 |
| Architektur | 20→32→24→7 (6 Policy-Logits + 1 Value-Kopf) |
| GPU | CUDA (nvidia) |

**Reward-Änderungen gegenüber Run 001** (angepasst vor diesem Lauf):

| Größe | Run 001 | Run 002 |
|---|---|---|
| MCTS Fress-Bonus | 0.3 | **0.5** |
| Self-Play Fressen | +1.0 | **+3.0** |
| Self-Play Lebenskosten | −0.005/Tick | **−0.01/Tick** |

Ziel: Futter-Priorisierung stärken, da Run 001 eine „kreisende" Überlebensstrategie
erlernt hatte (hohe Spiellänge, niedriger Score).

---

## 2. Lernkurve

| Iteration | Policy-Loss | Value-Loss | ~game_len | Bemerkung |
|---|---|---|---|---|
| 0   | 1.572 | 0.628 | 201  | Zufälliges Netz |
| 5   | 0.718 | 0.112 | 512  | Rasanter Anstieg |
| 10  | 0.285 | 0.198 | 566  | |
| 20  | 0.125 | 0.076 | 577  | |
| 50  | 0.062 | 0.097 | 639  | |
| 92  | 0.023 | 0.018 | **705** | **Peak game_len** |
| 120 | 0.002 | 0.057 | 627  | Rückgang nach Peak |
| 150 | 0.009 | 0.019 | 658  | Einpendeln |
| 1999| 0.000 | 0.000 | 659  | Vollständig konvergiert |

Policy-Loss erreicht ~0.000 ab Iteration ≈160 und bleibt dort für die
restlichen 1 840 Iterationen. Die mittlere Spiellänge peaked bei ~705
(iter 92), fiel danach zurück auf ~658 und blieb dort stabil — bei einem
Limit von 800 Ticks entspricht das einer Tick-Auslastung von 82%.

---

## 3. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks, sims=32.

| Topologie  | Run 001 | **Run 002** | Δ |
|------------|--------:|------------:|--:|
| Walls Ø    |   37.60 |   **19.58** | −47.9 % |
| Periodic Ø |   54.28 |   **44.26** | −18.5 % |

**Starke Regression gegenüber Run 001.** Run 002 wird deshalb
**nicht** als `best.mlp` eingecheckt — das Embedded-Netz bleibt Run 001.

---

## 4. Vergleich mit anderen Methoden

| Methode       | Walls Ø | Periodic Ø |
|---------------|--------:|-----------:|
| MLP-GA 005    |   91.40 |     125.12 |
| NEAT run-001  |   38.92 |      56.84 |
| PPO run-001   |   38.24 |      58.62 |
| AZ run-001    |   37.60 |      54.28 |
| DQN run-001   |   22.96 |      48.18 |
| **AZ run-002**| **19.58** | **44.26** |

Run 002 ist die schlechteste AlphaZero-Variante und fällt sogar hinter DQN zurück.

---

## 5. Analyse

### 5.1 Warum Regression trotz mehr Budget?

- **Policy-Kollaps nach dem Peak (iter 92)**: game_len fiel von 705 auf 627
  und pendelte sich bei 658 ein. Das Netz hatte sein Optimum früh gefunden
  und anschließend overfittet auf spezifische Training-Muster.

- **Zu wenig Training-Diversität durch `--max-ticks 800`**: Run 001 erlaubte
  1 500 Ticks — die Spiele hatten mehr Zeit, viele Futter-Ereignisse zu
  zeigen. Mit 800 Ticks endet ein Spiel, bevor der Gegner die
  Board-Komplexität (lange Schlange, voller Raum) wirklich erfahren kann.

- **Circling schlimmer als in Run 001**: avg Ticks 4 470 (Run 002) vs
  implizit weniger in Run 001 (game_len ~787 bei max-ticks 1 500). Das
  Netz überlebt sehr lange, isst aber weniger als 20 Äpfel in 4 000+ Ticks.
  Trotz verdreifachtem Futter-Reward (+3.0) blieb das Kreisverhalten erhalten.

- **Zu viele Iterationen**: Konvergenz bei ~iter 150, danach 1 850 nutzlose
  Iterationen. Das Netz wurde nicht schlechter, aber es lernte auch nichts
  Neues — frühe Abbruchoption fehlt.

- **Reward-Imbalanz**: Mit γ=0.99 und living_cost=−0.01 verliert ein Spiel
  mit 800 Ticks 8.0 Punkte allein durch Lebenskosten. Futter-Fressen
  bringt +3.0. Das System belohnt also eher kurze Spiele mit frühem
  Fressen als langes Überleben mit gelegentlichem Fressen.

### 5.2 Einfluss der Reward-Änderung

Der dreifache Futter-Reward (+3.0) allein hat das Kreisproblem nicht gelöst.
MCTS mit eat=0.5 sucht aggressiver nach Futter, aber die gelernte Policy
(die das Netz einbettet) ist trotzdem risikoavers. Das deutet darauf hin,
dass die Ursache im Suchbudget oder in der Trainingsdiversität liegt,
nicht im Reward.

---

## 6. Schlussfolgerungen für Run 003

- `--max-ticks 1 500` wiederverwenden (wie Run 001), oder höher (2 000+)
- Frühes Stoppen (checkpoint bei Peak game_len, nicht am Ende)
- `--iterations 300–500` statt 2 000 — Konvergenz ist schnell
- Alternative: Reward-Shaping zurückbauen auf Run-001-Werte und stattdessen
  MCTS-Budget erhöhen (`--sims 64+`) als Qualitätshebel

---

## 7. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-002/best.mlp` | Gewichte Run 002 (nicht deployed) |
| `training-out/az-run-002/train.log` | Vollständige Lernkurve (2 000 iter) |
| `crates/snake-core/assets/alphazero/best.mlp` | **Run 001** bleibt embedded |
