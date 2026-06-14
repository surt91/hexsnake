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
| Startarchitektur | 20 Inputs + 1 Bias → 6 Outputs, vollverbunden (27 Knoten) |
| Budget | ~14,4 M Spiel-Evaluierungen |

**Begründung**: NEAT startet mit einer minimalen, vollverbundenen Topologie
und lässt die Netzstruktur durch Mutation wachsen. Erster Lauf mit den
Standardparametern aus der Guide-Datei.

---

## 2. Bug-Fix: Crossover konnte Zyklen erzeugen

Beim ersten Lauf (derselbe Seed) paniktate der Trainer bei gen 330 mit
`"genome connections are not feed-forward"`. Ursache: Im Crossover werden
deaktivierte Verbindungen mit 25 % Wahrscheinlichkeit reaktiviert, ohne
zu prüfen, ob dadurch ein Zyklus entsteht. `creates_cycle` prüft nur
*aktivierte* Verbindungen — ein deaktivierter Rückkanten-Pfad entgeht der
Prüfung.

**Fix**: Nach dem Crossover führt `is_feed_forward()` (Kahn-Topo-Sort über
aktivierte Kanten) eine Vollprüfung durch. Bei erkanntem Zyklus wird das
Elternteil mit der höheren Fitness zurückgegeben. Commit `956bfd4`.

---

## 3. Lernkurve

| Generation | Best Fitness | Mean Fitness | Spezies | Knoten (Champion) |
|---|---|---|---|---|
| 0      | 825    | 330   | 1 | 27 |
| 500    | 4773   | 2427  | 1 | ~40 |
| 1 000  | 4872   | 2885  | 1 | 64 |
| 1 500  | 5796   | 3157  | 1 | ~80 |
| 2 000  | **6088** | 3229 | 1 | 86 |
| 2 500  | 5873   | 2927  | 1 | 105 |
| 2 999  | 5737   | 3067  | 1 | 111 |
| **best** | **6443** | — | — | — |

**Beobachtung**: Fitness-Peak bei gen ~2000, danach leichter Rückgang.
Konstant eine einzige Spezies über den gesamten Lauf — `--compat 3.0`
ist zu hoch für diese Netzgröße (zu wenig Speziationsschutz für neue
Topologien).

---

## 4. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 8 000 Ticks.

| Topologie  | Smoke-Run | **Run 001** | Δ |
|------------|----------:|------------:|--:|
| Walls Ø    |     28.04 |   **38.92** | +38.8 % |
| Periodic Ø |     59.36 |   **56.84** | −4.2 % |

Walls verbessert, Periodic leicht schlechter. Das deutet auf leichte
Überanpassung an Walls (oder auf Zufallsschwankungen bei nur 50 Spielen).

---

## 5. Checkpoint-Vergleich

Gemessen mit 50 Spielen, max. 4 000 Ticks.

| Checkpoint | Walls | Periodic | Knoten |
|------------|------:|---------:|-------:|
| gen 500    | 29.66 | 54.44    | ~40    |
| gen 1 000  | 30.00 | 53.10    | 64     |
| gen 2 000  | 38.24 | **61.24** | 86    |
| gen 2 900  | 34.88 | 61.74    | 105    |
| best.neat  | **38.92** | 56.84 | —  |

Gen 2000 ist der stärkste Checkpoint auf Periodic (61.24); das `best.neat`
(nach Fitness-Optimum gewählt) spielt besser auf Walls.

---

## 6. Beobachtungen

- **Keine Speziation**: Alle 3000 Generationen nur eine Spezies. Der
  `--compat 3.0`-Schwellenwert ist zu hoch — neue Topologien werden sofort
  in die Hauptspezies aufgenommen und konkurrieren direkt mit dem Champion.
  Für Run 002 `--compat 1.5` oder `--compat 2.0` testen.

- **Topologie wächst kontinuierlich**: Von 27 auf 112 Knoten in 3000 Gen
  (add_node dominiert). Kein Rückgang trotz mangelnder Speziation — das
  deutet auf stabile positive Selektion für mehr Knoten.

- **Fitness-Plateau nach gen 2000**: Mean-Fitness stagniert zwischen 2900
  und 3200 über 1500 Generationen. Ohne Speziationsschutz konsolidiert die
  Population zu früh.

- **MLP-GA deutlich stärker**: MLP-GA 005 erreicht Walls 91.4 / Periodic 125.1
  — mehr als doppelt so stark. NEAT hat hier konzeptionellen Vorteil (flexible
  Topologie), braucht aber weit mehr Budget und bessere Speziations-Parameter.

- **Bug-Fund**: Der Crossover-Zyklus-Bug wäre in Produktion schwer aufgefallen
  (sporadische Panik nach 300+ Gen). Er war latent seit der ersten NEAT-Impl.
  und tritt erst auf, wenn die Topologie komplex genug für Zyklus-Szenarien ist.

---

## 7. Fazit

Run 001 verbessert Walls deutlich (+39 %), verliert aber leicht auf Periodic.
Die Hauptursache für die begrenzten Ergebnisse ist die fehlende Speziation
(alle in einer Spezies): neue Topologien werden sofort von bestehenden
Champions verdrängt, bevor sie sich einspielen können. NEAT's eigentlicher
Vorteil — Nischen-Schutz für neue Strukturen — kann sich nicht entfalten.

**Für Run 002** empfohlen:
- `--compat 1.5` für mehr Speziation
- `--add-node 0.02`, `--add-conn 0.08` (mehr Verbindungen, weniger Knoten)
- Budget erhöhen: 5000 Gen oder größere Population

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/neat/best.neat` | Eingecheckt (Run 001 best) |
| `training-out/neat-run-001/best.neat` | Bestes Genom |
| `training-out/neat-run-001/fitness.csv` | Lernkurve |
| `training-out/neat-run-001/gen_*.neat` | Checkpoints alle 100 Gen |
