# Training Report: Neural Net GA — Run 005 (Alle Verbesserungen, großes Budget)

**Datum**: 2026-06-13  
**Ziel**: Kombination aller drei Verbesserungen aus Run 002–004 mit signifikant
größerem Trainingsbudget. Run 004 hat gezeigt, dass das größere Netz bei gleichem
Budget nicht gewinnt — dieser Run testet, ob es mit fairer Evaluationsanzahl
pro Parameter besser abschneidet.

---

## 1. Setup — alle Verbesserungen kombiniert

| Parameter            | Run 004 (Referenz)   | **Run 005 (alle 3 Optionen)** |
|----------------------|---------------------|-------------------------------|
| `--generations`      | 2000                | **5000** (+150 %)             |
| `--population`       | 256                 | **512** (+100 %)              |
| `--games`            | 24                  | 24                            |
| `--max-ticks`        | 3000                | 3000                          |
| `--sigma`            | 0.08                | **0.06** (feiner, großes Netz)|
| `--seed`             | 2                   | **3**                         |
| `--mixed`            | ja                  | ja                            |
| `--checkpoint-every` | 100                 | **250**                       |
| Feature-Vektor       | 20 (A+B)            | 20 (A+B, identisch)           |
| Architektur          | 20→32→24→6          | 20→32→24→6 (identisch)        |
| Budget               | 512.000 Evaluationen| **3.840.000 (7.5×)**          |
| Evals/Parameter      | ~317                | **~2380**                     |

**Begründung der Hyperparameter:**

- **5000 Generationen**: Run 004 hatte ~317 Evals/Parameter — zu wenig für 1614
  Parameter. Zielwert ~2000+/Parameter (wie Run 003 mit 618 Params hatte ~828).
- **Population 512**: Breitere Exploration im größeren Suchraum; hilft die
  Mean-Fitness-Lücke zu schließen, die in Run 003/004 beobachtet wurde.
- **Sigma 0.06** (statt 0.08): Leicht niedrigere Mutationsstärke für präzisere
  Suche im 1614-Parameter-Raum; noch ausreichend hoch für Exploration.
- **Checkpoint alle 250 Gen**: Bei 5000 Gen sonst zu viele Dateien (20 statt 50).

Alle anderen Verbesserungen bereits im Code:
- Mixed Training (Run 002)
- Kontinuierliches Food-Feature + globale Distanz (Run 003)
- Architektur 20→32→24→6 (Run 004)

---

## 2. Motivation: Evals-pro-Parameter als Metrik

Run 004 scheiterte nicht an der Architektur, sondern am Budget-Mismatch:

| Run  | Params | Total Evals   | Evals/Param |
|------|--------|---------------|-------------|
| 003  | 618    | 512.000       | 828         |
| 004  | 1614   | 512.000       | 317         |
| **005** | **1614** | **3.840.000** | **2380**  |

Run 005 gibt dem großen Netz ~7× mehr Evaluationen als Run 004 und ~3× mehr als
Run 003 (relativ zur Parameteranzahl). Das sollte einen fairen Vergleich ermöglichen.

---

## 3. Smoke-Run

Run 004 Smoke (selbe Architektur):
```
gen    0  best     30.00  mean     30.00
```
Pipeline validiert; Run 005 startet direkt mit vollem Budget.

Gen 0 beobachtet: `best 983.97, mean 329.88` — besser als Run 004 Gen 0 (790.88)
dank Population 512 (breitere Startstreuung).

---

## 4. Lernkurve

<!-- Wird nach Abschluss aus fitness.csv befüllt -->

| Generation | Best Fitness | Mean Fitness | Bemerkung |
|---|---|---|---|
| 0    | 983.97  | 329.88  | Pop 512: besserer Start als Run 004 (790) |
| 250  | _todo_  | _todo_  | |
| 500  | _todo_  | _todo_  | |
| 1000 | _todo_  | _todo_  | |
| 2000 | _todo_  | _todo_  | Run 004 best bei 2000 Gen: 8602 — Vergleichspunkt |
| 3000 | _todo_  | _todo_  | |
| 4000 | _todo_  | _todo_  | |
| 4999 | _todo_  | _todo_  | |
| best | _todo_  | —       | |

---

## 5. Benchmark-Ergebnis

<!-- Nach Abschluss befüllen -->

```bash
cp training-out/run-005/best.mlp crates/snake-core/assets/neural-net-ga/best.mlp
cargo run --release -p snake-core --example benchmark 50 10000
```

### Vergleich aller Runs

| Topologie    | Run 001 | Run 002 | Run 003 | Run 004 | **Run 005** | Δ zu 004 | Δ zu 003 |
|--------------|--------:|--------:|--------:|--------:|------------:|---------:|---------:|
| Walls Ø      |   80.60 |   63.72 |   70.32 |   71.30 |      _todo_ |   _todo_ |   _todo_ |
| Periodic Ø   |    6.08 |   85.32 |   90.42 |   88.44 |      _todo_ |   _todo_ |   _todo_ |

**Hypothesen:**
- Run 005 sollte Run 003 auf beiden Achsen übertreffen (faires Budget für großes Netz)
- Walls-Ziel: > 75 (zwischen Run 001 und 003)
- Periodic-Ziel: > 92 (über Run 003)

---

## 6. Checkpoint-Vergleich (50 Partien, 3000 Ticks)

<!-- Nach Abschluss befüllen — Checkpoints alle 250 Gen -->

| Checkpoint | Walls 005 | Periodic 005 | Walls 004 | Periodic 004 | Δ Walls | Δ Periodic |
|------------|----------:|-------------:|----------:|-------------:|--------:|-----------:|
| gen 250    | _todo_    | _todo_       | ≈55 (interpoliert) | ≈66 | _todo_ | _todo_ |
| gen 500    | _todo_    | _todo_       | 64.48     | 68.44        | _todo_  | _todo_     |
| gen 1000   | _todo_    | _todo_       | 72.66     | 81.12        | _todo_  | _todo_     |
| gen 1500   | _todo_    | _todo_       | 74.00     | 77.92        | _todo_  | _todo_     |
| gen 2000   | _todo_    | _todo_       | 71.30 (final Run 004) | 88.44 | _todo_ | _todo_ |
| gen 2500   | _todo_    | _todo_       | —         | —            | _todo_  | _todo_     |
| gen 3000   | _todo_    | _todo_       | —         | —            | _todo_  | _todo_     |
| gen 4000   | _todo_    | _todo_       | —         | —            | _todo_  | _todo_     |
| best (5000)| _todo_    | _todo_       | —         | —            | _todo_  | _todo_     |

---

## 7. Beobachtungen (laufend)

<!-- Wird während und nach dem Lauf ergänzt -->

### Gen-0-Beobachtung: Population 512 hilft sofort

Gen 0 best = 983 vs. Run 004 Gen 0 = 790. Die doppelte Populationsgröße
findet in der zufälligen Startpopulation bessere Individuen. Das erwartet
man bei gleichem Random-Seed (hier Seed 3 vs. 2), aber das Muster
ist konsistent mit theoretischer Erwartung.

---

## 8. Fazit (nach Abschluss ausfüllen)

_Hat das größere Budget die Hypothese bestätigt — schlägt Run 005 Run 003?_  
_Wo liegt das Plateau bei 5000 Generationen?_  
_Ist 20→32→24→6 + A+B + Budget das neue Optimum?_

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/neural-net-ga/best.mlp` | Eingechecktes Netz (nach Deployment) |
| `training-out/run-005/best.mlp` | Bestes Netz des Laufs |
| `training-out/run-005/fitness.csv` | Lernkurve (nicht eingecheckt) |
| `training-out/run-005/gen_*.mlp` | Checkpoints alle 250 Gen (21 Dateien) |
