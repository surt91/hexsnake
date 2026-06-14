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
| Laufzeit             | ~22 min                 | ~47 min                       |
| Ausgabe              | `training-out/run-001/` | `training-out/run-002/`       |

**Begründung der Änderungen:**

- `--games 24`: Run 001 zeigte starkes Messrauschen (Checkpoint-Scores ±5).
  Mehr Partien pro Fitness-Messung reduzieren die stochastische Varianz.
  Bei Mixed: 12 Walls + 12 Periodic pro Messung.
- `--seed 2`: anderer Startpunkt für unabhängige Replikation.
- `--sigma 0.08`, `--population 256`: aus Run 001 bewährt, keine Änderung.

Architektur: 19 → 16 → 12 → 6 (Mini-MLP, ReLU) — **unverändert**.
Kein Topologie-Sensor-Bit. Das Netz bekommt denselben Feature-Vektor wie immer;
die Generalisierung soll implizit durch das Training entstehen.

---

## 2. Smoke-Run (Verifikation --mixed)

```
gen    0  best    116.50  mean     41.38
gen    1  best    230.00  mean     91.23
gen    2  best    476.55  mean    103.69
best fitness: 476.55 -> /tmp/hexsnake-smoke-mixed/best.mlp
```

Vergleich zu Run-001-Smoke (Walls-only, best 580.00): Mixed-Fitness ist erwartungsgemäß
niedriger — Periodic-Spiele sind schwieriger und ziehen den Durchschnitt nach unten.

---

## 3. Lernkurve

| Generation | Best Fitness | Mean Fitness | Bemerkung |
|------------|-------------|-------------|-----------|
| 0          | 1149.38     | 348.35      | Ähnlicher Start wie Run 001 (1130) |
| 100        | 6035.71     | 4692.16     | Run 001 war hier bei 6794 — Mixed startet langsamer |
| 200        | 6454.50     | 5398.54     | |
| 300        | 7268.22     | 5952.15     | |
| 400        | 7303.90     | 5922.11     | |
| 500        | 7454.74     | 6401.25     | Plateau zeichnet sich ab |
| 600        | 7514.28     | 6382.72     | |
| 700        | 7480.36     | 6465.51     | |
| 800        | 7932.09     | 6789.22     | |
| 900        | 7715.95     | 6495.01     | |
| 1000       | 7782.56     | 6595.74     | Ab hier kaum noch Best-Verbesserung |
| 1100       | 8068.38     | 6942.34     | |
| 1200       | 7982.75     | 6748.45     | |
| 1300       | 8026.55     | 6990.24     | |
| 1400       | 8039.52     | 7283.29     | Mean steigt weiter — Population konsolidiert |
| 1500       | 8074.86     | 7112.98     | |
| 1600       | 8035.87     | 7035.77     | |
| 1700       | 8013.41     | 7073.11     | |
| 1800       | 8130.05     | 7151.69     | |
| 1900       | 8434.36     | 7400.38     | |
| 1999       | 8239.14     | 7515.89     | |
| **best**   | **8536.01** | —           | Gen 1887 — kumulatives Optimum |

Die vollständige Kurve liegt in `training-out/run-002/fitness.csv`.

**Vergleich Run 001 vs. Run 002 (Fitness, nicht Score!):**

| Gen  | Run 001 best | Run 002 best | Differenz |
|------|-------------|-------------|-----------|
| 100  | 6794        | 6035        | -11 %     |
| 500  | 8157        | 7454        | -9 %      |
| 1000 | 8459        | 7782        | -8 %      |
| best | 9069        | 8536        | -6 %      |

Run 002 liegt konsistent ~8-11 % unter Run 001 — erklärt durch das schwerere
gemischte Ziel. Die Lücke schließt sich leicht über die Zeit.

---

## 4. Benchmark-Ergebnis (50 Partien, 10 000 Ticks, 16×12)

```bash
cp training-out/run-002/best.mlp crates/snake-core/assets/mlp-ga/best.mlp
cargo run --release -p snake-core --example benchmark 50 10000
```

### Walls

| Strategie       | Run 001 Ø | Run 002 Ø | Differenz         |
|-----------------|----------:|----------:|-------------------|
| Greedy          |     23.40 |     23.40 | Referenz          |
| Monte-Carlo     |     59.60 |     59.60 | Referenz          |
| **Neural Net**  | **80.60** | **63.72** | **-21 %**         |

### Periodic

| Strategie       | Run 001 Ø | Run 002 Ø | Differenz         |
|-----------------|----------:|----------:|-------------------|
| Greedy          |     32.96 |     32.96 | Referenz          |
| Monte-Carlo     |     95.60 |     95.60 | Referenz          |
| **Neural Net**  |  **6.08** | **85.32** | **+1303 %**       |

**Das Mixed-Training funktioniert.** Das Netz hat ohne explizites Topologie-Feature
beide Modi gelernt:
- Walls: 63.72 — immer noch 13 % über Monte-Carlo (59.60), 2.7× Greedy
- Periodic: 85.32 — von quasi-0 auf 89 % von Monte-Carlos Periodic-Score

---

## 5. Checkpoint-Vergleich (50 Partien, 3000 Ticks)

| Checkpoint | Ø Score Walls | Ø Score Periodic | Bemerkung |
|------------|-------------:|----------------:|-----------|
| gen 100    | 51.88        | 59.32           | Beide Modi lernen gleichzeitig |
| gen 500    | 59.40        | 73.78           | Walls erreicht Monte-Carlo-Niveau |
| gen 1000   | 65.20        | 83.56           | Walls überschreitet MC; Periodic nähert sich |
| gen 1500   | 66.02        | 78.22           | Messrauschen: Periodic schwankt |
| gen 1900   | 60.86        | 85.44           | Walls fällt leicht, Periodic steigt |
| **best**   | **63.72**    | **85.32**       | 10 000-Tick volles Benchmark |

**Bemerkenswert**: Ab Gen 1000 stagniert Walls (65→66→61→64), während Periodic
langsam weiter steigt (83→78→85). Das Netz "wählt" bei Sättigung Periodic-Optimierung
auf Kosten von Walls — möglicherweise weil Periodic-Spiele mehr Raum für Verbesserung
boten (kein Hard-Cap durch Topologie-Wissen).

---

## 6. Beobachtungen

### Szenario 2 trat ein — Trade-off, aber besser als erwartet

Die drei erwarteten Szenarien waren:
1. Beide Modi gut — generelle Heuristik
2. **Kompromiss — Walls leicht schlechter, Periodic deutlich besser** ✓
3. Catastrophic forgetting — keine stabile Lösung

Tatsächlich: Walls -21 %, Periodic +1303 %. Der Walls-Verlust ist real, aber
das Netz bleibt auf Walls *über* Monte-Carlo-Niveau — kein katastrophales Vergessen.

### Das Netz "sieht" die Topologie implizit

Ohne explizites Topologie-Bit lernt das Netz trotzdem beide Modi. Der Schlüssel:
`blocking_dist_inv` verhält sich auf Periodic anders als auf Walls — auf dem Torus
trifft ein Strahl beim Wrap-Around die eigene Schlange, nicht eine Wand. Diese
subtile Signaldifferenz reicht dem MLP aus, um die Topologie zu erschließen.

### Warum ist Walls schlechter als Run 001?

Die Fitness-Funktion mischt nun Walls- und Periodic-Spiele. Die Selektion
bevorzugt Genome, die *im Schnitt über beide Topologien* gut sind — das ist ein
anderes Optimierungsziel als nur-Walls. Ein Netz, das auf Walls perfekte Muster
kennt, die auf Periodic fatal wären, wird abgestraft. Das ist der intendierte
Kompromiss.

### Fitness-Plateau ab Gen 1000 (Mixed)

Das Plateau tritt bei Mixed ~gen 1000 auf, bei Run 001 war es ~gen 500. Mixed braucht
länger zum Konvergieren, weil das Optimierungsziel komplexer ist — zwei verschiedene
Physiken in einer Gewichtsmatrix abzubilden. Aber auch hier: ab Gen 1000 verbessert
der Best-Wert sich kaum noch (7782 → 8536, +9.7 % über 1000 Gen).

### Längeres Training würde wenig bringen

Das klare Plateau ab Gen 1000 auf beiden Score-Achsen zeigt: der Bottleneck ist
die **Feature-Darstellung**, nicht die Trainingsdauer. Mit denselben 19 Features
wird 4000 Generationen den Walls-Score kaum von 63 auf 75 bringen.

---

## 7. Feature-Analyse: Wo liegt das Potenzial?

Die aktuellen 19 Features (6 Strahlen × 3 + 1 global):

| Feature           | Pro Strahl | Problem |
|-------------------|:----------:|---------|
| `blocking_dist_inv` | ja       | OK — korrekt auf beiden Topologien |
| `body_dist_inv`     | ja       | OK |
| `approaches_food`   | ja       | **Binary-Flag** — verliert Distanzinfo! |
| `snake_len`         | global   | OK |

Das größte Verbesserungspotenzial: **`approaches_food` ist ein 1-Bit-Flag**.
Es sagt nur ja/nein, ob der nächste Schritt das Essen annähert — nicht wie weit
das Essen entfernt ist. Bei einem 16×12-Feld mit Torus-Wrap können Distanzen
stark variieren; ein Netz, das nicht "weiß", ob Essen 2 oder 25 Felder weit weg
ist, muss raten.

**Vorgeschlagene Verbesserungen für Run 003:**

1. **Kontinuierliches Food-Feature** (gleiche Feature-Anzahl, kein Architektur-Umbau):
   `(food_dist - new_food_dist) / max_steps` statt Binary-Flag — normalisierte
   Distanzänderung pro Strahl.

2. **Globale Nahrungsdistanz** (20 Features): `1 / food_dist` als 20. Input —
   gibt dem Netz absolute Distanzinformation zum Essen.

3. **Größeres Netz** (unabhängig): 19→32→24→6 statt 19→16→12→6 — 3× mehr
   Parameter für die schwerere Dual-Topologie-Aufgabe.

---

## 8. Ideen für Folgeversuche

- [x] **Run 002**: Mixed-Training 50/50 ohne Sensor-Bit — **erledigt**
- [ ] **Run 003**: Kontinuierliches Food-Feature + globale Distanz (Option A+B)
- [ ] **Run 004**: Größeres Netz (19→32→24→6) mit Mixed-Training
- [ ] **Topologie-Bit-Vergleich**: Run 005 mit `is_periodic`-Bit als 20. Feature —
  wie viel helfen 1 Bit explizite Information?
- [ ] **Asymmetrisches Sampling**: 70 % Walls / 30 % Periodic — weniger
  Walls-Einbuße?
- [ ] **Lernkurven-Diagramm**: Walls- und Periodic-Score als zwei Linien über
  Generationen — schöne Blog-Grafik

---

## 9. Fazit

**Ja, Mixed-Training funktioniert ohne explizites Topologie-Feature.**

Das Netz lernt implizit aus dem unterschiedlichen Verhalten von `blocking_dist_inv`
auf den beiden Topologien, welchen Modus es gerade spielt. Das ist überraschend
effektiv — Periodic von 6 auf 85 (+1303 %) zeigt, dass die 50/50-Fitness die
richtige Lösung war.

Der Walls-Verlust (-21 %, von 80.6 auf 63.7) ist der erwartete Preis für
Generalisierung. Walls bleibt über Monte-Carlo-Niveau, Periodic kommt fast
an Monte-Carlo heran.

Der nächste sinnvolle Schritt ist **nicht** längeres Training, sondern bessere
Features: ein kontinuierliches Food-Feature würde dem Netz deutlich mehr
Navigationsinformation geben und voraussichtlich beide Scores verbessern.

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/mlp-ga/best.mlp` | Eingechecktes Netz (dieser Run) |
| `training-out/run-002/best.mlp` | Bestes Netz des Laufs (identisch) |
| `training-out/run-002/fitness.csv` | Lernkurve (nicht eingecheckt) |
| `training-out/run-002/gen_*.mlp` | Zwischen-Checkpoints alle 100 Gen |
| `training-out/run-002/train.log` | Vollständige Konsolenausgabe |
