# Training Report: Neural Net GA — Run 004 (Größeres Netz 20→32→24→6)

**Datum**: 2026-06-13  
**Ziel**: Option C — mehr Netzkapazität (2.6× Parameter) mit A+B-Features und
Mixed-Training. Test ob die Mean-Fitness-Lücke aus Run 003 durch mehr Kapazität
geschlossen wird und ob beide Topologie-Scores weiter steigen.

---

## 1. Setup

| Parameter            | Run 003 (A+B, klein) | Run 004 (A+B, groß)      |
|----------------------|---------------------|--------------------------|
| `--generations`      | 2000                | 2000                     |
| `--population`       | 256                 | 256                      |
| `--games`            | 24                  | 24                       |
| `--max-ticks`        | 3000                | 3000                     |
| `--sigma`            | 0.08                | 0.08                     |
| `--seed`             | 2                   | 2                        |
| `--mixed`            | ja                  | ja                       |
| Feature-Vektor       | 20                  | 20 (identisch)           |
| Architektur          | 20→16→12→6          | **20→32→24→6**           |
| Parameter            | ~618                | **~1614 (2.6×)**         |

**Architektur-Änderung (`snake-core/src/nn/mod.rs`):**
```
alt: pub const HIDDEN: [usize; 2] = [16, 12];
neu: pub const HIDDEN: [usize; 2] = [32, 24];
```

Parameter-Kalkulation:
- 20→16→12→6: (20×16+16) + (16×12+12) + (12×6+6) = 336+204+78 = **618**
- 20→32→24→6: (20×32+32) + (32×24+24) + (24×6+6) = 672+792+150 = **1614**

---

## 2. Smoke-Run

```
gen    0  best     30.00  mean     30.00
gen    1  best     30.00  mean     30.00
gen    2  best     30.00  mean     30.00
best fitness: 30.00
```

Fitness 30 = nur Ticks × 0.1 bei 300 max-ticks × 2 games — keine Äpfel gegessen.
Bei 3 Generationen mit Population 12 hat ein großes Netz mit random Gewichten
wenig Chance, zufällig zu fressen. Smoke-Run validiert nur Pipeline (Build,
Format, Parse), nicht Lernverhalten.

---

## 3. Lernkurve

<!-- Wird nach Abschluss aus fitness.csv befüllt -->

| Generation | Best Fitness | Mean Fitness | Bemerkung |
|---|---|---|---|
| 0   | 790.88  | 324.74  | Niedrig — großes Netz startet zufälliger |
| 1   | 1869.80 | 387.21  | Schneller Sprung in Gen 1 |
| 680 | 7580    | ~5730   | Halbzeit-Stand: leicht unter Run 003 (7766 bei Gen 700) |
| 100 | _todo_  | _todo_  | |
| 500 | _todo_  | _todo_  | |
| 1000| _todo_  | _todo_  | |
| 1500| _todo_  | _todo_  | |
| 1999| _todo_  | _todo_  | |
| best| _todo_  | —       | |

---

## 4. Benchmark-Ergebnis

<!-- Nach Abschluss befüllen -->

### Vergleich Run 001–004

| Topologie    | Run 001 | Run 002 | Run 003 | **Run 004** | Δ zu 003 |
|--------------|--------:|--------:|--------:|------------:|---------:|
| Walls Ø      |   80.60 |   63.72 |   70.32 |      _todo_ |   _todo_ |
| Periodic Ø   |    6.08 |   85.32 |   90.42 |      _todo_ |   _todo_ |

---

## 5. Checkpoint-Vergleich (50 Partien, 3000 Ticks)

<!-- Nach Abschluss befüllen -->

| Checkpoint | Walls 004 | Periodic 004 | Walls 003 | Periodic 003 | Δ Walls | Δ Periodic |
|---|---:|---:|---:|---:|---:|---:|
| gen 100  | _todo_ | _todo_ | 33.26 | 0.00  | _todo_ | _todo_ |
| gen 500  | _todo_ | _todo_ | 58.42 | 78.76 | _todo_ | _todo_ |
| gen 1000 | _todo_ | _todo_ | 62.34 | 77.48 | _todo_ | _todo_ |
| gen 1500 | _todo_ | _todo_ | 67.94 | 82.76 | _todo_ | _todo_ |
| gen 1900 | _todo_ | _todo_ | 69.42 | 85.98 | _todo_ | _todo_ |
| best     | _todo_ | _todo_ | 70.32 | 90.42 | _todo_ | _todo_ |

---

## 6. Hypothesen

### Hypothese 1: Größeres Netz = langsamerer Start, besseres Ende

Bei Run 003 startete gen-0 bei 901; Run 004 startet bei 791. Mehr Parameter =
größerer Suchraum = mehr Generationen nötig. Wenn Run 003 ein "schlechter Start,
besseres Ende"-Muster zeigte (gen 100 Periodic=0, dann Aufholen), könnte Run 004
das noch extremer zeigen.

### Hypothese 2: Mean-Fitness-Lücke schließt sich

Run 003 hatte durchgängig niedrigere Mean-Fitness als Run 002, was auf einen
breiten, schwer zu konsolidierenden Suchraum hindeutete. Mit mehr Netzkapazität
könnte die Population gute Strategien schneller finden — oder umgekehrt, der noch
größere Suchraum macht es noch schwerer.

### Hypothese 3: Beide Topologien profitieren — oder Overfitting auf eine

Das Netz hat nun genug Kapazität, für jede Topologie separate "Schaltkreise"
zu lernen. Wenn es das tut, sollten beide Scores deutlich steigen. Wenn es
stattdessen overfittet (eine Topologie dominiert die Fitness), könnte die andere
leiden.

---

## 7. Fazit (nach Abschluss ausfüllen)

_Hat das größere Netz geholfen?_  
_Welche Hypothese trat ein?_  
_Ist 20→32→24→6 das neue Optimum, oder war 20→16→12→6 mit A+B bereits ausreichend?_

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/neural-net-ga/best.mlp` | Eingechecktes Netz (nach Deployment) |
| `training-out/run-004/fitness.csv` | Lernkurve (nicht eingecheckt) |
| `training-out/run-004/gen_*.mlp` | Checkpoints alle 100 Gen |
