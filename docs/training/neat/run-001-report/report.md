# Training Report: NEAT — Run 001

**Datum**: 2026-06-14  
**Ziel**: Erster produktiver NEAT-Lauf mit gemischten Rändern.
Baseline bisher: Smoke-Run (Walls Ø 28, Periodic Ø 59).

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Methode | NEAT (NeuroEvolution of Augmenting Topologies) |
| `--generations` | 3 000 |
| `--population` | 300 |
| `--games` | 16 |
| `--max-ticks` | 3 000 |
| `--mixed` | ja (50/50 Walls/Periodic) |
| `--compat` | 3.0 |
| `--add-conn` | 0.06 |
| `--add-node` | 0.03 |
| `--seed` | 1 |
| `--checkpoint-every` | 100 |
| Startarchitektur | 20 Inputs → 6 Outputs, vollverbunden (120 P) |
| Budget | ~14,4 M Spiel-Evaluierungen |

**Begründung**: NEAT beginnt mit einer kleinen, vollverbundenen Topologie
und wächst durch Mutation. Anders als beim MLP-GA gibt es keinen festen
Parameter-Count — die Netzgröße ist ein Hyperparameter, der sich evolviert.

---

## 2. Lernkurve

| Generation | Best Fitness | Mean Fitness | Spezies | Bemerkung |
|---|---|---|---|---|
| 0      | _todo_ | _todo_ | _todo_ | |
| 500    | _todo_ | _todo_ | _todo_ | |
| 1 000  | _todo_ | _todo_ | _todo_ | |
| 2 000  | _todo_ | _todo_ | _todo_ | |
| 3 000  | _todo_ | _todo_ | _todo_ | |
| best   | _todo_ | — | — | |

---

## 3. Benchmark-Ergebnis

| Topologie  | Smoke-Run | **Run 001** | Δ |
|------------|----------:|------------:|--:|
| Walls Ø    |     28.04 |      _todo_ | _todo_ |
| Periodic Ø |     59.36 |      _todo_ | _todo_ |

---

## 4. Checkpoint-Vergleich

| Checkpoint  | Walls | Periodic | Knoten (ca.) | Kanten (ca.) |
|-------------|------:|---------:|-------------:|-------------:|
| gen 0 500   | _todo_ | _todo_  | _todo_       | _todo_       |
| gen 1 000   | _todo_ | _todo_  | _todo_       | _todo_       |
| gen 2 000   | _todo_ | _todo_  | _todo_       | _todo_       |
| gen 3 000   | _todo_ | _todo_  | _todo_       | _todo_       |

---

## 5. Beobachtungen

_Wird nach Abschluss ergänzt._

---

## 6. Fazit

_Übertrifft NEAT den MLP-GA? Wie stark wächst die Topologie?_  
_Zeigt Speziation messbaren Vorteil gegenüber einfacher ES?_

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/neat/best.neat` | Eingecheckt nach Deployment |
| `training-out/neat-run-001/best.neat` | Bestes Netz |
| `training-out/neat-run-001/fitness.csv` | Lernkurve |
| `training-out/neat-run-001/gen_*.neat` | Checkpoints alle 100 Gen |
