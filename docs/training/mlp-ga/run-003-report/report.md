# Training Report: Neural Net GA — Run 003 (Verbesserte Food-Features A+B)

**Datum**: 2026-06-13  
**Ziel**: Kontinuierliches Food-Feature (A) + globale Nahrungsdistanz (B) testen.
Mixed-Training 50/50, Architektur 20→16→12→6 (ein Input mehr als Run 002).

---

## 1. Setup

| Parameter            | Run 002 (Mixed)  | Run 003 (A+B)             |
|----------------------|-----------------|---------------------------|
| `--generations`      | 2000            | 2000                      |
| `--population`       | 256             | 256                       |
| `--games`            | 24              | 24                        |
| `--max-ticks`        | 3000            | 3000                      |
| `--sigma`            | 0.08            | 0.08                      |
| `--seed`             | 2               | 2                         |
| `--mixed`            | ja              | ja                        |
| Feature-Vektor       | 19              | **20**                    |
| Architektur          | 19→16→12→6      | **20→16→12→6**            |
| Laufzeit             | ~47 min         | ~48 min                   |

**Feature-Änderungen (`snake-core/src/nn/features.rs`):**

**A — Kontinuierliches Food-Approach-Feature** (ersetzt Binary-Flag):
```
alt: 1.0 wenn Nachbar näher am Essen, sonst 0.0
neu: (food_dist - neighbor_food_dist) / food_dist
```
- Positiv = Bewegung nähert ans Essen; Negativ = entfernt sich
- Betrag wächst je näher das Essen ist (→ 1.0 wenn Essen direkt daneben)
- 0.0 bei Wand (kein Nachbar)

**B — Globale Nahrungsdistanz** (20. Feature):
`food_dist / max_steps` ∈ [0, 1]

---

## 2. Smoke-Run

```
gen    0  best    130.00  mean     42.50
gen    1  best    475.80  mean     92.15
gen    2  best    421.50  mean    125.12
best fitness: 475.80
```

Vergleichbar mit Run 002 Smoke (476.55) — neues Feature-Format validiert.

---

## 3. Lernkurve

| Generation | Best Fitness | Mean Fitness | Bemerkung |
|------------|-------------|-------------|-----------|
| 0          | 901.73      | 328.99      | Tiefer Start — neues Feature noch ungenutzt |
| 100        | 5720.28     | 3720.25     | Langsamer als Run 002 (6035) — neues Feature braucht Anlernzeit |
| 200        | 6468.94     | 4482.44     | |
| 300        | 6752.28     | 4412.40     | |
| 400        | 6879.57     | 5173.79     | |
| 500        | 7466.71     | 5157.03     | Run 002 bei 7454 — gleichgezogen |
| 600        | 7542.61     | 5491.06     | |
| 700        | 7766.03     | 5500.79     | |
| 800        | 7871.50     | 5483.90     | |
| 900        | 7892.93     | 5789.99     | |
| 1000       | 8032.02     | 5819.27     | Run 002 bei 7782 — Run 003 überholt |
| 1100       | 8105.75     | 5745.47     | |
| 1200       | 8178.75     | 6002.21     | |
| 1300       | 8318.74     | 6153.04     | |
| 1400       | 8261.12     | 6631.07     | |
| 1500       | 8381.12     | 6850.21     | |
| 1600       | 8382.60     | 6824.34     | |
| 1700       | 8556.46     | 6970.58     | |
| 1800       | 8523.06     | 6992.54     | |
| 1900       | 8592.93     | 7033.72     | |
| 1999       | 8601.22     | 7167.53     | |
| **best**   | **8981.57** | —           | Gen 1887 (kumulatives Optimum) |

Run 002 best war 8536 — Run 003 +5.2 %. Die neuen Features brauchen mehr Generationen
zum Einprägen, überholen aber ab Gen ~1000.

---

## 4. Benchmark-Ergebnis (50 Partien, 10 000 Ticks, 16×12)

### Vergleich Run 001 / 002 / 003

| Topologie    | Run 001 (Walls) | Run 002 (Mixed) | **Run 003 (A+B)** | Δ zu 002  |
|--------------|----------------:|----------------:|------------------:|----------:|
| Walls Ø      |           80.60 |           63.72 |         **70.32** | **+10 %** |
| Periodic Ø   |            6.08 |           85.32 |         **90.42** |  **+6 %** |
| Walls max    |              93 |              83 |                85 |           |
| Periodic max |              46 |             109 |               114 |           |

A+B verbessert **beide** Topologien gleichzeitig:
- Walls erholt sich von 63.7 auf 70.3 — 11 % über Monte-Carlo (59.6)
- Periodic nähert sich Monte-Carlo (95.6) auf 5 %

---

## 5. Checkpoint-Vergleich (50 Partien, 3000 Ticks)

| Checkpoint | Walls 003 | Periodic 003 | Walls 002 | Periodic 002 | Δ Walls | Δ Periodic |
|------------|----------:|-------------:|----------:|-------------:|--------:|-----------:|
| gen 100    | **33.26** | **0.00**     | 51.88     | 59.32        | -36 %   | -100 %     |
| gen 500    | 58.42     | 78.76        | 59.40     | 73.78        | -2 %    | +7 %       |
| gen 1000   | 62.34     | 77.48        | 65.20     | 83.56        | -4 %    | -7 %       |
| gen 1500   | 67.94     | 82.76        | 66.02     | 78.22        | +3 %    | +6 %       |
| gen 1900   | 69.42     | 85.98        | 60.86     | 85.44        | +14 %   | +1 %       |
| **best**   | **70.32** | **90.42**    | 63.72     | 85.32        | **+10 %**|**+6 %**  |

---

## 6. Beobachtungen

### Das spektakuläre Gen-100-Ergebnis: Periodic = 0

Bei gen 100 erzielt Run 003 auf Periodic Ø **0.00** — das Netz läuft in
Endlosschleifen und frisst kaum Äpfel. Run 002 war dort bereits bei 59.

Die Erklärung: Das neue food_approach-Feature hat einen anderen Wertebereich ([-1, 1]
statt [0, 1]) und erfordert andere Gewichte, um nützlich zu werden. In den ersten
100 Generationen hat die Population noch keinen Weg gefunden, das neue Signal zu
nutzen — das alte Netz mit Binary-Flag hatte eine "natürliche" Nullhypothese
(Flag nie gesetzt = ignoriere Essen), das neue Feature kann aktiv schaden,
wenn die Gewichte falsch gepolt sind.

Ab Gen 500 ist dieser Nachteil vollständig aufgeholt (78.76 vs. 73.78 für 002).
Das ist ein klassischer **„schlechter Start, besseres Ende"**-Verlauf.

### Verzögerter Überholeffekt

Run 003 liegt bis Gen ~900 unter Run 002, überholt dann und liegt am Ende +5 %
drüber. Mehr Informationsgehalt im Feature-Vektor braucht mehr Generationen um
"verstanden" zu werden — die Evolutionsstrategie muss Gewichtskombinationen finden,
die das kontinuierliche Signal korrekt interpretieren, statt nur ein Bit zu testen.

### Beide Topologien profitieren

Entscheidend: Die Feature-Verbesserung hilft Walls (+10 %) **und** Periodic (+6 %)
gleichzeitig. Das continuous food_approach-Feature verbessert die Navigationsstrategie
in beiden Topologien, weil das eigentliche Problem (schlechte Nahrungsortung) auf
beiden Gittern dasselbe war.

### Mean-Fitness bleibt hinter Run 002

Die Mean-Fitness von Run 003 ist in fast allen Generationen niedriger als Run 002
(z.B. Gen 1000: 5819 vs. 6595). Die **Population** lernt langsamer; nur das
**Eliten-Individuum** überholt. Das deutet auf einen breiteren Suchraum durch das
neue Feature hin — die Populationsmasse braucht länger, um gute Strategien zu
konsolidieren. Interessant für spätere Tuning-Versuche (höheres `--population` oder
`--games`).

---

## 7. Theoretischer Hintergrund: Warum ist das Feature besser?

Das alte Binary-Flag `approaches_food` war equivalent zu:
```
score(direction) = 1 if food_dist(next_cell) < food_dist(head) else 0
```

Das neue kontinuierliche Feature ist:
```
score(direction) = (food_dist(head) - food_dist(next_cell)) / food_dist(head)
```

Der Unterschied:
1. **Amplitude**: Essen 1 Feld weg → Signal 1.0; Essen 20 Felder weg → Signal 0.05.
   Das Netz "weiß" jetzt, wie dringend es ist, in diese Richtung zu gehen.
2. **Negatives Signal**: -0.1 bedeutet "dieser Schritt entfernt leicht vom Essen" —
   früher war das undifferenziert 0.0 zusammen mit "keine Nachbarzelle (Wand)".
3. **Globale Distanz (Feature B)**: Das Netz kennt jetzt auch den absoluten Abstand,
   nicht nur die relative Richtungsänderung — wichtig für Pfadplanung bei weitem Essen.

---

## 8. Ideen für Folgeversuche

- [x] **Run 003**: A+B Features — **erledigt, beide Topologien +10 % / +6 %**
- [ ] **Run 004**: Größeres Netz 20→32→24→6 mit A+B — läuft gerade
- [ ] **Mehr Generationen mit A+B**: Da Run 003 bis Gen 900 noch lernt, könnte
  3000 Gen den Endwert weiter verbessern
- [ ] **Höhere Population mit A+B**: Mean bleibt hinter Run 002 — Population 512
  könnte helfen, die Masse schneller zu konsolidieren
- [ ] **food_approach mit Clamping**: Werte können theoretisch < -1 sein bei
  Torus-Pfaden; `clamp(-1, 1)` zur Sicherheit
- [ ] **Weitere Global-Features**: Anzahl sicherer Moves, Flood-Fill-Schätzung

---

## 9. Fazit

**A+B verbessert beide Topologien** gegenüber Run 002:
- Walls: 63.7 → 70.3 (+10 %)
- Periodic: 85.3 → 90.4 (+6 %)

Der überraschendste Befund: gen 100 mit Periodic=0 zeigt, dass das neue Feature
initial "schlechter" ist als das Binary-Flag — aber langfristig deutlich besser.
Das ist ein starkes Argument dafür, bei Feature-Änderungen genug Generationen zu
trainieren, bevor man entscheidet.

Die Mean-Fitness-Lücke legt nahe, dass das Feature-Potenzial noch nicht voll
ausgeschöpft ist: Run 004 (größeres Netz) testet, ob mehr Kapazität hilft,
die neuen Features besser zu nutzen.

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/run-003/best.mlp` | Bestes Netz des Laufs (Architektur 20→16→12→6) |
| `training-out/run-003/fitness.csv` | Lernkurve (nicht eingecheckt) |
| `training-out/run-003/gen_*.mlp` | Checkpoints alle 100 Gen |
