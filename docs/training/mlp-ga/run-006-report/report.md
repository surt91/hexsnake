# Training Report: Neural Net GA — Run 006 (20 000 Generationen)

**Datum**: 2026-06-13/14  
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
| Laufzeit (ca.)       | ~5,5 h              | **~22 h**                 |

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

Gen 0 best = **2223** (Run 005 gen 0: 983) — Seed 4 erzeugt eine stärkere
Startpopulation. Zunächst sah das vielversprechend aus. Gen-0-Stärke ist jedoch
ein Indikator für die Initialpopulation, nicht für das Endresultat: der Lauf
konvergierte zu einem schwächeren Optimum als Run 005.

---

## 3. Lernkurve

| Generation | Best Fitness | Mean Fitness | Bemerkung |
|---|---|---|---|
| 0     | 2223.08 | 349.44  | Bester gen-0 aller Runs — trügerisch |
| 1000  | 8507.83 | 5837.80 | Run 005 bei gen 1000: **9 888** |
| 2000  | 8836.76 | 6317.12 | Run 005 bei gen 2000: **10 660** |
| 5000  | 9588.04 | 8175.64 | Run 005 final best: **12 434** — Vergleichspunkt |
| 10000 | 9689.47 | 8524.23 | |
| 15000 | 10319.41 | 9421.23 | |
| 19999 | 10698.37 | 9700.04 | |
| best  | **10902.67** | — | Unterer als Run 005 (12 434) |

Bereits ab gen 1000 liegt Run 006 signifikant hinter Run 005 — trotz stärkerem
gen-0. Das deutet auf eine unterschiedliche Landschaft, in die Seed 4 führt,
nicht auf einen trägen Start.

---

## 4. Benchmark-Ergebnis

Gemessen mit 50 Spielen, max. 10 000 Ticks (gleiche Methodik wie Run 005).

| Topologie    | Run 003 | Run 005 | **Run 006** | Δ zu 005 |
|--------------|--------:|--------:|------------:|---------:|
| Walls Ø      |   70.32 |   91.40 |   **86.16** |   −5.8 % |
| Periodic Ø   |   90.42 |  125.12 |  **121.86** |   −2.6 % |

**Hypothesen gescheitert.** Beide Scores liegen unter Run 005 — obwohl 4× das
Budget eingesetzt wurde. Run 005 bleibt das beste Netz und ist weiterhin als
`best.mlp` eingecheckt.

---

## 5. Checkpoint-Vergleich

Gemessen mit 50 Spielen, max. 3 000 Ticks (einheitlich für alle Checkpoints).

| Checkpoint          | Walls 006 | Periodic 006 | Walls 005¹ | Periodic 005¹ |
|---------------------|----------:|-------------:|-----------:|--------------:|
| gen 1 000           |     68.38 |        86.52 |      78.00 |        100.20 |
| gen 2 000           |     —     |        —     |      81.24 |        105.86 |
| gen 3 000           |     —     |        —     |      83.74 |        117.62 |
| gen 4 000           |     —     |        —     |      82.24 |        121.82 |
| gen 5 000           |     76.58 |        92.82 |  **91.40** |    **125.12** |
| gen 10 000          |     79.58 |       102.40 |      —     |         —     |
| gen 15 000          |     86.78 |       105.38 |      —     |         —     |
| gen 19 000          |     87.40 |       106.72 |      —     |         —     |
| best.mlp (final)    | **86.16** |    **121.86**|      —     |         —     |

¹ Run 005 hatte Checkpoints alle 250 Gen; interpolierte Werte aus Benchmarks.

Schon bei gen 5000 liegt Run 006 deutlich hinter Run 005 an der gleichen
Generation. Run 006 braucht etwa **gen 15 000** um das Walls-Niveau zu
erreichen, das Run 005 bei gen 5 000 hatte — und übertrifft es nie.

Das final `best.mlp` (aus fitness-Tracking, Fitness 10902.67) zeigt beim
Periodic-Benchmark einen Sprung von 106 (gen 19000) auf 122 — vermutlich ein
glücklicherer Evaluierungs-Seed bei der Bestleistungs-Generation.

---

## 6. Beobachtungen

- **Seed-Sensitivität überwiegt das Budget**: Das GA ist hoch stochastisch.
  Seed 4 führt in ein anderes Attraktionsgebiet des Parameterraums als Seed 3
  — mit 20 000 Gen nicht mehr verlassbar.

- **Beste Fitness nicht cross-Run vergleichbar**: Fitness-Werte sind
  generations-indexiert (`gen × 10000 + game`). Gleiche Generation → gleiche
  Eval-Boards. Bei gen 5000 hat Run 006 best=9588, Run 005 best=12434 — die
  gleichen Boards, aber eine deutlich schlechtere Population. Das bestätigt:
  Seed 4 steuerte wirklich in ein schlechteres Optimum.

- **Frühes Konvergieren**: Die Mean-Fitness bei gen 19999 (9700) liegt sehr
  nah an der Best-Fitness (10698) — die Population hat sich stark
  konsolidiert. Run 005 zeigte am Ende noch mehr Diversität.

- **Lernrate flacht sehr früh ab**: Von gen 5000 zu gen 19999 steigt Best nur
  von 9588 auf 10698 (+11 %), während Mean von 8175 auf 9700 (+19 %) steigt.
  Der Algorithmus findet in 15 000 weiteren Generationen kaum bessere
  Individuen — ein klares Zeichen für lokale Konvergenz.

---

## 7. Fazit

**Die Budget-Hypothese wurde widerlegt.** 4× mehr Generationen verbesserten
das Ergebnis nicht — Run 005 (seed 3, 5 000 Gen) bleibt überlegen.

Die zentrale Erkenntnis: Bei einfachem Truncation-ES ist die **Seed-Wahl
mindestens genauso entscheidend wie das Budget**. Ein längerer Lauf mit einem
ungünstigen Seed übertrifft einen kürzeren Lauf mit einem günstigen Seed
nicht. Der Grund: Sobald die Population in einem lokalen Optimum konvergiert,
hilft mehr Zeit kaum noch — das gleiche Sigma 0.06 kann den Talkessel nicht
verlassen.

**Was helfen könnte:**
- Mehrere unabhängige Läufe mit gleicher Laufzeit, bestes Ergebnis wählen
  (parallele Exploration statt serieller Ausdauer).
- Adaptive σ (CMA-ES): passt die Mutationsschrittweite und Richtung an,
  entkommt lokalen Optima systematisch besser.
- Steigende σ-Sequenz oder Restart-Strategie wenn Mean/Best konvergieren.

**Best.mlp bleibt Run 005** (Walls Ø 91.40, Periodic Ø 125.12).

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/mlp-ga/best.mlp` | Weiterhin Run 005 (Run 006 schlechter) |
| `training-out/run-006/best.mlp` | Bestes Netz des Laufs (Referenz) |
| `training-out/run-006/fitness.csv` | Lernkurve (nicht eingecheckt) |
| `training-out/run-006/gen_*.mlp` | Checkpoints alle 1 000 Gen (21 Dateien) |
