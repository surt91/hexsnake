# Training Report: Neural Net GA — Run 006 (20 000 Generationen)

**Datum**: 2026-06-13  
**Ziel**: Zeigen, dass Run 005 noch kein Plateau hatte und mehr Budget weitere
Verbesserung bringt. 4× das Budget von Run 005.

---

## 1. Setup

| Parameter            | Run 005              | **Run 006**               |
|----------------------|---------------------|---------------------------|
| `--generations`      | 5 000               | **20 000 (+300 %)**       |
| `--population`       | 512                 | 512                       |
| `--games`            | 24                  | 24                        |
| `--max-ticks`        | 3 000               | 3 000                     |
| `--sigma`            | 0.06                | 0.06                      |
| `--seed`             | 3                   | **4**                     |
| `--mixed`            | ja                  | ja                        |
| `--checkpoint-every` | 250                 | **1 000**                 |
| Feature-Vektor       | 20 (A+B)            | 20 (A+B)                  |
| Architektur          | 20→32→24→6 (~1614 P)| 20→32→24→6 (~1614 P)      |
| Budget               | 3,84 M Evals        | **15,36 M Evals (4×)**    |
| Evals/Parameter      | ~2 380              | **~9 520**                |
| Laufzeit (geschätzt) | ~5,5 h              | **~22 h**                 |

**Begründung der Hyperparameter:**

- `--sigma 0.06`: aus Run 005 bewährt, kein Grund zu ändern.
- `--population 512`: bewährt und ausreichend breit für den 1614-Param.-Raum.
- `--games 24`: 32 würde ~29 h ergeben — unverhältnismäßig.
- `--checkpoint-every 1000`: bei 20 k Gen wären 250er-Checkpoints 80 Dateien;
  1000er ergibt 21 übersichtliche Checkpoints.
- `--seed 4`: neuer Seed für unabhängige Replikation.

**Hypothese**: Run 005 zeigte bis gen 4750 steigenden Score (Walls 94, Periodic 136).
Mit 20 000 Gen sollte das Plateau erst deutlich später erreicht werden.
Ziele: **Walls > 95**, **Periodic > 140**.

---

## 2. Früher Verlauf

Gen 0 best = **2223** (Run 005 gen 0: 983) — Seed 4 hat eine stärkere Startpopulation.

---

## 3. Lernkurve

<!-- Wird nach Abschluss aus fitness.csv befüllt -->

| Generation | Best Fitness | Mean Fitness | Bemerkung |
|---|---|---|---|
| 0     | 2223.08 | 349.44 | Bester gen-0 aller Runs |
| 1000  | _todo_  | _todo_ | Run 005 bei gen 1000: 9888 |
| 2000  | _todo_  | _todo_ | Run 005 bei gen 2000: 10660 |
| 5000  | _todo_  | _todo_ | Run 005 final best: 12434 — Vergleichspunkt |
| 10000 | _todo_  | _todo_ | |
| 15000 | _todo_  | _todo_ | |
| 19999 | _todo_  | _todo_ | |
| best  | _todo_  | —      | |

---

## 4. Benchmark-Ergebnis

<!-- Nach Abschluss befüllen -->

| Topologie    | Run 003 | Run 005 | **Run 006** | Δ zu 005 |
|--------------|--------:|--------:|------------:|---------:|
| Walls Ø      |   70.32 |   91.40 |      _todo_ |   _todo_ |
| Periodic Ø   |   90.42 |  125.12 |      _todo_ |   _todo_ |

Hypothesen:
- Walls > 95 (Extrapolation aus 005-Checkpoint-Kurve)
- Periodic > 140

---

## 5. Checkpoint-Vergleich

<!-- Nach Abschluss befüllen — Checkpoints alle 1000 Gen -->

| Checkpoint  | Walls 006 | Periodic 006 | Walls 005¹ | Periodic 005¹ |
|-------------|----------:|-------------:|-----------:|--------------:|
| gen 1 000   | _todo_    | _todo_       | 78.00      | 100.20        |
| gen 2 000   | _todo_    | _todo_       | 81.24      | 105.86        |
| gen 3 000   | _todo_    | _todo_       | 83.74      | 117.62        |
| gen 4 000   | _todo_    | _todo_       | 82.24      | 121.82        |
| gen 5 000   | _todo_    | _todo_       | 91.40      | 125.12 (best) |
| gen 10 000  | _todo_    | _todo_       | —          | —             |
| gen 15 000  | _todo_    | _todo_       | —          | —             |
| gen 20 000  | _todo_    | _todo_       | —          | —             |

¹ Run 005 hatte Checkpoints alle 250 Gen; hier interpolierte Werte aus Benchmarks.

---

## 6. Beobachtungen (laufend)

_Wird während und nach dem Lauf ergänzt._

---

## 7. Fazit (nach Abschluss)

_Bestätigt 20 000 Gen die Budget-Hypothese aus Run 005?_  
_Wo liegt das echte Plateau?_  
_Lohnt es sich, noch länger zu trainieren?_

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/neural-net-ga/best.mlp` | Eingechecktes Netz (nach Deployment) |
| `training-out/run-006/best.mlp` | Bestes Netz des Laufs |
| `training-out/run-006/fitness.csv` | Lernkurve (nicht eingecheckt) |
| `training-out/run-006/gen_*.mlp` | Checkpoints alle 1 000 Gen (21 Dateien) |
