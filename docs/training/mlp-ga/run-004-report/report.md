# Training Report: Neural Net GA — Run 004 (Größeres Netz 20→32→24→6)

**Datum**: 2026-06-13  
**Ziel**: Option C — mehr Netzkapazität (2.6× Parameter) mit A+B-Features und
Mixed-Training. Testet ob das größere Netz die A+B-Features besser ausnutzt
und den Mean-Fitness-Rückstand aus Run 003 aufholt.

---

## 1. Setup

| Parameter            | Run 003 (A+B, klein)    | Run 004 (A+B, groß)      |
|----------------------|-------------------------|--------------------------|
| `--generations`      | 2000                    | 2000                     |
| `--population`       | 256                     | 256                      |
| `--games`            | 24                      | 24                       |
| `--max-ticks`        | 3000                    | 3000                     |
| `--sigma`            | 0.08                    | 0.08                     |
| `--seed`             | 2                       | 2                        |
| `--mixed`            | ja                      | ja                       |
| Feature-Vektor       | 20 (A+B)                | 20 (A+B, identisch)      |
| Architektur          | 20→16→12→6              | **20→32→24→6**           |
| Parameter            | ~618                    | **~1614 (2.6×)**         |
| Laufzeit             | ~48 min                 | ~55 min                  |

Parameter-Kalkulation:
- 20→16→12→6: (20×16+16) + (16×12+12) + (12×6+6) = 336+204+78 = **618**
- 20→32→24→6: (20×32+32) + (32×24+24) + (24×6+6) = 672+792+150 = **1614**

---

## 2. Smoke-Run

```
gen    0  best     30.00  mean     30.00
gen    1  best     30.00  mean     30.00
gen    2  best     30.00  mean     30.00
```

Fitness 30 = nur Ticks × 0.1 bei 300 max-ticks — kein Apfel gegessen.
Ein zufällig initialisiertes 1614-Parameter-Netz spielt bei Population 12
in 3 Generationen quasi-zufällig. Validiert nur die Pipeline.

---

## 3. Lernkurve

| Generation | Best Fitness | Mean Fitness | Bemerkung |
|------------|-------------|-------------|-----------|
| 0          | 790.88      | 324.74      | Niedriger als Run 003 (901) — größerer Suchraum |
| 100        | 6432.84     | 4392.83     | **Schneller als Run 003 (5720)** — mehr Kapazität hilft früh |
| 200        | 7024.16     | 4398.97     | |
| 300        | 6773.07     | 4454.62     | Rückfall — Suchraum breiter, Rauschen höher |
| 400        | 7005.03     | 4512.19     | |
| 500        | 7160.03     | 4818.24     | Run 003 war bei 7466 — jetzt zurückgefallen |
| 600        | 7580.19     | 5273.38     | |
| 700        | 7982.90     | 5925.08     | Run 003 bei 7766 — gleichauf |
| 800        | 8125.53     | 5663.09     | |
| 900        | 7927.75     | 5630.44     | |
| 1000       | 7803.08     | 5716.90     | Run 003 bei 8032 — Run 004 hinten |
| 1100       | 7903.10     | 5692.30     | |
| 1200       | 8356.30     | 5910.63     | |
| 1300       | 8203.19     | 6027.13     | |
| 1400       | 8135.97     | 5860.69     | |
| 1500       | 8180.79     | 6030.82     | Run 003 bei 8381 — Lücke bleibt |
| 1600       | 8355.32     | 6029.24     | |
| 1700       | 8241.03     | 6001.42     | |
| 1800       | 8063.63     | 6010.11     | |
| 1900       | 8147.35     | 6266.64     | |
| 1999       | 8213.73     | 6079.00     | |
| **best**   | **8602.76** | —           | Run 003 best: 8981 — **Run 004 ist schlechter** |

---

## 4. Benchmark-Ergebnis (50 Partien, 10 000 Ticks, 16×12)

### Vergleich aller Runs

| Topologie    | Run 001 | Run 002 | Run 003 | **Run 004** | Δ zu 003  |
|--------------|--------:|--------:|--------:|------------:|----------:|
| Walls Ø      |   80.60 |   63.72 |   70.32 |   **71.30** | **+1.4 %** |
| Periodic Ø   |    6.08 |   85.32 |   90.42 |   **88.44** | **-2.2 %** |
| Walls max    |      93 |      83 |      85 |          84 |           |
| Periodic max |      46 |     109 |     114 |     **159** | **+39 %** |

**Run 004 ist gegenüber Run 003 ein Unentschieden** auf den Durchschnittswerten
(+1 % / -2 %, beides im Rauschbereich von 50 Spielen). Das größere Netz bringt
keinen klaren Gewinn bei diesem Trainingsbudget.

**Bemerkenswert**: Periodic max Score 159 (vs. 114 bei Run 003) — das große Netz
erreicht deutlich höhere Spitzenwerte, ist aber inkonsistenter. Hohe Varianz
statt stabiler Durchschnitt.

---

## 5. Checkpoint-Vergleich (50 Partien, 3000 Ticks)

| Checkpoint | Walls 004 | Periodic 004 | Walls 003 | Periodic 003 | Δ Walls | Δ Periodic |
|------------|----------:|-------------:|----------:|-------------:|--------:|-----------:|
| gen 100    | **50.40** | **64.22**    | 33.26     | 0.00         | **+52 %** | **+∞**  |
| gen 500    | 64.48     | 68.44        | 58.42     | 78.76        | +10 %   | -13 %      |
| gen 1000   | **72.66** | **81.12**    | 62.34     | 77.48        | +17 %   | +5 %       |
| gen 1500   | 74.00     | 77.92        | 67.94     | 82.76        | +9 %    | -6 %       |
| gen 1900   | 72.20     | 77.66        | 69.42     | 85.98        | +4 %    | -10 %      |
| **best**   | **71.30** | **88.44**    | 70.32     | 90.42        | +1.4 %  | -2.2 %     |

---

## 6. Beobachtungen

### Hypothese 1 bestätigt: Besserer Start, keine besseres Ende

Gen 100: Run 004 (50.40 / 64.22) deutlich besser als Run 003 (33.26 / 0.00).
Das größere Netz hat von Anfang an mehr Kapazität, gute Strategien für beide
Topologien gleichzeitig zu repräsentieren — kein Periodic-Einbruch bei gen 100.

Ab Gen 500 dreht sich das Bild aber um: Run 003 holt auf und überholt Run 004
auf Periodic (78.76 vs. 68.44). Das kleinere Netz findet in einem engeren
Suchraum effizienter gute Lösungen.

### Hypothese 2 nicht bestätigt: Mean-Fitness-Lücke bleibt

Run 003 hatte niedrigere Mean-Fitness als Run 002. Run 004 hat noch niedrigere
Mean-Fitness als Run 003 (z.B. Gen 1000: 5716 vs. 5819). Das größere Netz
macht den Konsolidierungs-Nachteil nicht besser, sondern schlimmer.

Ursache: Bei gleicher Population (256) und gleichem Sigma (0.08) haben
1614-Parameter-Mutationen eine geringere "Trefferquote" als 618-Parameter-Mutationen.
Das Populationsrauschen wächst mit dem Suchraum. Für einen fairen Vergleich
bräuchte Run 004 eine größere Population oder mehr Generationen.

### Hohe Varianz bei großem Netz

Periodic max 159 vs. 114 zeigt: Das große Netz kann gelegentlich sehr lange
und effizient spielen — wahrscheinlich weil es für spezifische Spielsituationen
bessere "Schaltkreise" gelernt hat. Aber es spielt inkonsistenter: der Durchschnitt
ist niedriger als Run 003.

Das ist ein klassisches **Bias-Varianz-Dilemma** in der Evolutionsstrategie:
- Kleines Netz: niedriger Bias, niedrige Varianz → konsistent gut
- Großes Netz: niedriger Bias, **hohe Varianz** → gelegentlich besser, oft schlechter

### Suchraum-Budget-Mismatch

2000 Generationen × Population 256 = 512.000 Individuen-Evaluationen.
Bei 618 Parametern: ~828 Evaluationen pro Parameter.
Bei 1614 Parametern: ~317 Evaluationen pro Parameter.

Das kleinere Netz bekommt mehr als 2.6× so viele "Versuche pro Parameter"
— das erklärt den Vorteil. Eine faire Vergleich bräuchte:
`2000 × (1614/618) ≈ 5220 Generationen` für Run 004, oder
`Population 670` bei gleichem Budget.

---

## 7. Was wäre nötig, damit Run 004 Run 003 schlägt?

Optionen (einzeln oder kombiniert):

| Ansatz | Erwarteter Effekt | Aufwand |
|--------|-------------------|---------|
| `--generations 5000` | Mehr Budget für großen Suchraum | 2.5× Laufzeit |
| `--population 512` | Bessere Exploration | 2× Laufzeit |
| `--sigma 0.04` (feinere Suche ab Gen 500) | Langsamere aber präzisere Konvergenz | Resume-Funktion nötig |
| Architektur 20→24→18→6 (Mittelweg) | ~900 Parameter, Kompromiss | Neuer Run |

---

## 8. Gesamtbild: Was haben Runs 001–004 gelernt?

| Run | Key Change | Walls | Periodic | Erkenntnis |
|-----|-----------|------:|--------:|-----------|
| 001 | Baseline Walls-only | 80.6 | 6.1 | Ohne Mixed: Torus-Blindheit |
| 002 | Mixed Training | 63.7 | 85.3 | Mixed funktioniert, Walls-Preis −21 % |
| 003 | Kontinuierl. Food-Feature | 70.3 | 90.4 | Bessere Features helfen beiden Modi |
| 004 | Größeres Netz | 71.3 | 88.4 | Kein Gewinn bei gleichem Budget |

**Bestes allround Netz: Run 003** (70.3 Walls / 90.4 Periodic).  
**Höchste Einzelleistung**: Run 004 (Periodic max 159).  
**Bester Walls-only**: Run 001 (80.6, aber Periodic versagt).

---

## 9. Ideen für nächste Schritte

- [ ] **Run 005: Run 004 mit mehr Budget** — 5000 Gen oder Pop 512, fairer Vergleich
- [ ] **Mittelweg-Architektur**: 20→24→18→6 (~900 Parameter) — Kompromiss zwischen
  618 und 1614
- [ ] **Zweistufiges Training**: Erst Run 003 (kleines Netz) bis Plateau, dann
  Gewichte auf großes Netz übertragen (Transfer Learning / Net2Net)
- [ ] **Sigma-Decay**: Start mit σ=0.12 (Exploration), ab Gen 1000 auf σ=0.04
  (Feinsuche) — würde großem Netz helfen
- [ ] **Blog-Diagramm**: Alle 4 Runs in einem Plot — Walls und Periodic als zwei
  Linien, alle Runs als verschiedene Farben

---

## 10. Fazit

**Das größere Netz (20→32→24→6) bringt bei gleichem Trainingsbudget keinen
signifikanten Gewinn** gegenüber 20→16→12→6 mit A+B-Features.

Der entscheidende Befund: Das Problem ist nicht die Netzkapazität, sondern das
Training-Budget relativ zur Parameteranzahl. Ein 618-Parameter-Netz wird mit
2000 Gen / Pop 256 deutlich besser "durchgesucht" als ein 1614-Parameter-Netz
mit demselben Budget.

Für Blog und Praxis ist **Run 003 der aktuelle Sieger**: es verbessert gegenüber
dem Baseline-Mixed (Run 002) beide Topologien signifikant, mit minimalem Mehraufwand
(ein anderes Feature, eine Zahl mehr im Feature-Vektor).

Das eingecheckte `best.mlp` entspricht Run 004, da die Architektur nun 20→32→24→6
ist. Für einen echten Produktions-Einsatz wäre ein Rückschritt auf 20→16→12→6
(Run 003 Architektur) mit `best.mlp = training-out/run-003/best.mlp` die bessere Wahl.

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/mlp-ga/best.mlp` | Smoke-Run-Platzhalter (20→32→24→6) |
| `training-out/run-004/best.mlp` | Bestes Netz des Laufs |
| `training-out/run-004/fitness.csv` | Lernkurve (nicht eingecheckt) |
| `training-out/run-004/gen_*.mlp` | Checkpoints alle 100 Gen |
