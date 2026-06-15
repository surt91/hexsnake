# Training Report: AlphaZero-light — Run 022

**Datum**: 2026-06-15
**Ziel**: Erste Untersuchung der Seed-Abhängigkeit mit *einem* Seed und höherem
Compute-Budget. Hypothese (Nutzer): Das Training hängt extrem vom Seed ab, weil
die Schlange am Anfang zu selten frisst und „im Kreis fahren" lernt; außerdem
verschlechtern sich die Ergebnisse mit längerer Trainingsdauer — beides riecht
nach einem Problem der Reward-/Checkpoint-Logik.

Baseline (deployed): Run 021 Seed 68 — Walls 40.72, Periodic 75.74, Avg 58.23
(200 Spiele).

---

## 1. Reward-/Setup-Analyse (vor dem Lauf)

- **Self-Play-Reward** (`self_play_with_rewards`): −0.005/Schritt, +`sp_eat`
  (1.0) beim Fressen, ±0.1·ΔFutterdistanz (Shaping), −1.0 Tod, +2.0 Sieg →
  diskontierter Return (γ=0.99), `tanh` als Value-Target.
- **MCTS-Kantenreward**: dichtes Schritt-Reward (Fress-Bonus 0.3 +
  Annäherungs-Shaping) im Suchbaum — verhindert Cold-Start-Kollaps.
- **Befund 1 (Kernproblem)**: `train_alphazero.py` wählte `best.mlp` über
  `avg_game_len = len(rows)/games` — also **Überlebenszeit**, nicht Score.
  Eine Schlange, die sicher im Kreis fährt, maximiert game_len und wird als
  „best" gespeichert. Genau das im `blog_notes.md` dokumentierte „best.mlp ≠
  bester Checkpoint". Das ist die direkte Ursache für „länger → schlechter".

## 2. Fix in diesem Lauf

`self_play_with_rewards`/`az_selfplay` geben jetzt Score und Ticks pro Spiel
zurück (`SelfPlayResult`); der Trainer wählt `best.mlp` nach **mittlerem
Self-Play-Score** statt game_len.

## 3. Setup

| Parameter | Wert |
|---|---|
| `--iterations` | 150 |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--max-ticks` | 1500 |
| `--seed` | 1 |
| Checkpoint-Auswahl | mittlerer Self-Play-Score |

## 4. Ergebnis

Self-Play-Score stieg monoton 0.8 → 72 (kein Kollaps), `best.mlp` = iter 148.
Aber `bench_mlp` (greedy, 8000 Ticks):

| Netz | Walls | Periodic | avg_ticks (Walls) |
|---|---:|---:|---:|
| Champion s68 | 40.04 | 67.40 | 1098 |
| Run 022 best (iter 148) | **19.42** | 47.48 | **4116 (kreist!)** |
| Run 022 final (iter 149) | 17.88 | 47.34 | 4564 |

## 5. Analyse / Lehre

Score-basierte Auswahl ist nötig, aber **nicht hinreichend**: Der mittlere
Self-Play-Score (stochastisch, bei 1500 Ticks gedeckelt) bleibt hoch, während
die *greedy* Policy auf den Benchmark-Boards im Kreis fährt (Walls avg_ticks
4116, Score 19). Self-Play-Score und Greedy-Benchmark sind entkoppelt.

→ **Nächster Schritt (Run 023)**: Checkpoint nach einem *greedy* Benchmark
auswählen, nicht nach Self-Play-Score.
