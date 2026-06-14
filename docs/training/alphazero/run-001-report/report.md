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
| Budget (ca.) | 80 × 128 × ~50 Züge × 24 Sims = ~12,3 M MCTS-Calls |

**Hinweis**: `embedded()` spielt mit `sims=24` — dieser Wert muss nach dem
Einbetten mit der Konstante in `alphazero.rs` übereinstimmen.

---

## 2. Lernkurve

| Iteration | Policy-Loss | Value-Loss | mean_return | Bemerkung |
|---|---|---|---|---|
| 0   | _todo_ | _todo_ | _todo_ | |
| 20  | _todo_ | _todo_ | _todo_ | |
| 40  | _todo_ | _todo_ | _todo_ | |
| 60  | _todo_ | _todo_ | _todo_ | |
| 80  | _todo_ | _todo_ | _todo_ | |

---

## 3. Benchmark-Ergebnis

| Topologie  | Smoke-Run | **Run 001** | Δ |
|------------|----------:|------------:|--:|
| Walls Ø    |     30.04 |      _todo_ | _todo_ |
| Periodic Ø |     53.76 |      _todo_ | _todo_ |

---

## 4. Vergleich mit anderen Methoden

| Methode     | Walls Ø | Periodic Ø |
|-------------|--------:|-----------:|
| MLP-GA 005  |   91.40 |     125.12 |
| NEAT run-001| _todo_  |     _todo_ |
| DQN run-001 | _todo_  |     _todo_ |
| PPO run-001 | _todo_  |     _todo_ |
| AZ run-001  | _todo_  |     _todo_ |

---

## 5. Beobachtungen

_Wird nach Abschluss ergänzt._

---

## 6. Fazit

_AlphaZero-light: Lohnt MCTS zur Inferenzzeit gegenüber reiner MLP-Policy?_  
_Wie wirkt sich die Suche (sims=24) auf die Spielstärke aus?_

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/alphazero/best.mlp` | Eingecheckt nach Deployment |
| `training-out/az-run-001/best.mlp` | Bestes Netz (Gradient-Training) |
| `training-out/az-run-001/training.log` | Lernkurve |
