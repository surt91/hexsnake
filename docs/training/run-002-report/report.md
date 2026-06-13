# Training Report: Neural Net GA — Run 002 (Mixed Boundary)

**Datum**: 2026-06-13  
**Ziel**: Netz trainieren, das auf beiden Topologien (Walls und Periodic) funktioniert,
durch 50/50-Mixed-Boundary-Fitness. Kein extra Sensor-Bit — die Generalisierung
soll allein durch das Training entstehen.

Direkter Vergleich zu Run 001 (Walls-only), um den Topologie-Generalisierungs-Effekt
zu messen.

---

## 1. Setup

| Parameter            | Run 001 (Walls)         | Run 002 (Mixed) — dieser Lauf |
|----------------------|-------------------------|-------------------------------|
| `--generations`      | 2000                    | 2000                          |
| `--population`       | 256                     | 256                           |
| `--games`            | 12                      | **24** (+100 %)               |
| `--max-ticks`        | 3000                    | 3000                          |
| `--sigma`            | 0.08                    | 0.08                          |
| `--seed`             | 1                       | **2**                         |
| `--mixed`            | nein                    | **ja** (50 % Walls / 50 % Periodic) |
| `--checkpoint-every` | 100                     | 100                           |
| Hardware             | 32 CPU-Kerne            | 32 CPU-Kerne                  |
| Ausgabe              | `training-out/run-001/` | `training-out/run-002/`       |

**Begründung der Änderungen:**

- `--games 24`: Run 001 zeigte starkes Messrauschen in der 50-Spiele-Benchmark
  (Checkpoint-Scores schwankten ±5). Mehr Partien pro Fitness-Messung reduzieren
  die stochastische Varianz bei der Selektion. Bei Mixed sind es 12 Walls + 12
  Periodic pro Messung.
- `--seed 2`: anderer Startpunkt für unabhängige Replikation.
- `--sigma 0.08`, `--population 256`: aus Run 001 bewährt, keine Änderung.

Architektur: 19 → 16 → 12 → 6 (Mini-MLP, ReLU) — **unverändert**. Kein
Topologie-Sensor-Bit. Das Netz bekommt denselben Feature-Vektor wie immer;
die Aufgabe ist es, implizit zu lernen, mit beiden Topologien umzugehen.

---

## 2. Smoke-Run (Verifikation)

```
gen    0  best    116.50  mean     41.38
gen    1  best    230.00  mean     91.23
gen    2  best    476.55  mean    103.69
best fitness: 476.55 -> /tmp/hexsnake-smoke-mixed/best.mlp
```

Vergleich zu Run-001-Smoke (Walls-only, best 580.00): Die Mixed-Fitness ist
erwartungsgemäß niedriger — Periodic-Spiele sind schwieriger.

---

## 3. Lernkurve

<!-- Wird nach Abschluss aus fitness.csv befüllt -->

| Generation | Best Fitness | Mean Fitness | Bemerkung |
|---|---|---|---|
| 0   | _todo_ | _todo_ | |
| 100 | _todo_ | _todo_ | |
| 500 | _todo_ | _todo_ | |
| 1000| _todo_ | _todo_ | |
| 1500| _todo_ | _todo_ | |
| 1999| _todo_ | _todo_ | |
| best| _todo_ | —       | Kumulatives Optimum |

**Hinweis zur Vergleichbarkeit**: Die Fitness-Werte sind nicht direkt mit Run 001
vergleichbar — der Mixed-Lauf bewertet auf einem schwereren gemischten Ziel.

---

## 4. Benchmark-Ergebnis

<!-- Nach Abschluss befüllen -->

```bash
cp training-out/run-002/best.mlp crates/snake-core/assets/neural-net-ga/best.mlp
cargo run --release -p snake-core --example benchmark 50 10000
```

### Walls

| Strategie   | Run 001 Ø | Run 002 Ø | Differenz |
|-------------|----------:|----------:|----------:|
| Greedy      |     23.40 |     23.40 | Referenz  |
| Monte-Carlo |     59.60 |     59.60 | Referenz  |
| Neural Net  |     80.60 |    _todo_ |    _todo_ |

### Periodic

| Strategie   | Run 001 Ø | Run 002 Ø | Differenz |
|-------------|----------:|----------:|----------:|
| Greedy      |     32.96 |     32.96 | Referenz  |
| Monte-Carlo |     95.60 |     95.60 | Referenz  |
| Neural Net  |      6.08 |    _todo_ |    _todo_ |

---

## 5. Checkpoint-Vergleich (50 Partien, 3000 Ticks)

<!-- Nach Abschluss befüllen -->

| Checkpoint | Ø Score Walls | Ø Score Periodic | Bemerkung |
|------------|-------------:|----------------:|-----------|
| gen 100    | _todo_       | _todo_          | |
| gen 500    | _todo_       | _todo_          | |
| gen 1000   | _todo_       | _todo_          | |
| gen 1500   | _todo_       | _todo_          | |
| gen 1900   | _todo_       | _todo_          | |
| best       | _todo_       | _todo_          | |

---

## 6. Beobachtungen

### Fitness-Niveau Mixed vs. Walls-only

Die Mixed-Fitness-Werte sind nicht direkt mit Run-001 vergleichbar:
jede Partie bewertet eine Mischung aus einfacheren (Walls) und schwereren
(Periodic) Spielen. Der absolute Fitness-Wert wird niedriger sein als
in Run 001, auch wenn das Netz insgesamt besser ist.

### Erwartung: Walls-Score sinkt, Periodic-Score steigt

Das zentrale Experiment: Kann das Netz ohne explizites Topologie-Feature
beide Modi lernen? Mögliche Szenarien:

1. **Beide Modi gut** — das Netz findet eine generelle Heuristik (z. B.
   „geh zur nächsten Apfelrichtung, meide gefährliche Nachbarn"), die auf
   beiden Gittern funktioniert.
2. **Kompromiss** — Walls leicht schlechter als Run 001 (-10 %), Periodic
   deutlich besser als Run 001 (+40 %). Trade-off durch gemischte Fitness.
3. **Catastrophic forgetting** — keine stabile Lösung gefunden, da die
   Feature-Kodierung für Walls-Wände kontraproduktiv auf dem Torus ist.

<!-- Nach Abschluss: Welches Szenario trat ein? -->

### Topologie-Blindheit des Feature-Vektors

Die 19 Sensor-Features enthalten `wall_dist_*`-Werte, die bei Periodic
die Torus-Distanz verwenden (korrekt implementiert). Das bedeutet: Das Netz
bekommt semantisch korrekte Distanzen in beiden Modi, aber keinen
Hinweis darauf, *welche* Topologie gerade aktiv ist. Das ist die
interessante Frage — reicht das Signal aus den Distanzwerten, um die
Topologie implizit zu erkennen?

---

## 7. Ideen und Folgeversuche

- [ ] **Run 003: Topologie-Feature**: 1-Bit `is_periodic` als 20. Eingabe —
  direkter Vergleich mit diesem Lauf
- [ ] **Sigma-Sweep bei Mixed**: 0.04 / 0.08 / 0.16 — konvergiert Mixed
  schneller mit höherem Sigma?
- [ ] **Benchmark über Checkpoints visualisieren**: beide Achsen (Walls und
  Periodic) als zwei Linien im gleichen Diagramm — schöne Blog-Grafik
- [ ] **Asymmetrisches Sampling**: 70 % Walls / 30 % Periodic — weniger
  Einbußen auf Walls bei mehr Periodic-Generalisierung?

---

## 8. Fazit (nach Abschluss ausfüllen)

_Konnte das Netz ohne Topologie-Feature beide Modi lernen?_
_Wie groß ist der Trade-off gegenüber Run 001 auf Walls?_
_Was ist die nächste sinnvolle Iteration?_

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/neural-net-ga/best.mlp` | Eingechecktes Netz (nach Deployment) |
| `training-out/run-002/best.mlp` | Bestes Netz des Laufs |
| `training-out/run-002/fitness.csv` | Lernkurve (nicht eingecheckt) |
| `training-out/run-002/gen_*.mlp` | Zwischen-Checkpoints alle 100 Gen |
| `training-out/run-002/train.log` | Vollständige Konsolenausgabe |
