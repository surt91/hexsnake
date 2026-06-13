# Training Report: Neural Net GA — Run 003 (Verbesserte Food-Features)

**Datum**: 2026-06-13  
**Ziel**: Kontinuierliches Food-Feature (A) + globale Nahrungsdistanz (B) testen.
Mixed-Training 50/50, selbe Architektur wie Run 002 (20→16→12→6 nach Feature-Erweiterung).

---

## 1. Setup und Änderungen gegenüber Run 002

| Parameter            | Run 002 (Mixed)         | Run 003 (A+B)            |
|----------------------|-------------------------|--------------------------|
| `--generations`      | 2000                    | 2000                     |
| `--population`       | 256                     | 256                      |
| `--games`            | 24                      | 24                       |
| `--max-ticks`        | 3000                    | 3000                     |
| `--sigma`            | 0.08                    | 0.08                     |
| `--seed`             | 2                       | 2                        |
| `--mixed`            | ja                      | ja                       |
| Feature-Vektor       | 19 (binary food-flag)   | **20 (kontinuierlich)**  |
| Architektur          | 19→16→12→6              | **20→16→12→6**           |

**Feature-Änderungen (beide in `snake-core/src/nn/features.rs`):**

**A — Kontinuierliches Food-Approach-Feature** (ersetzt Binary-Flag):  
War: `1.0 wenn Nachbar näher am Essen, sonst 0.0`  
Neu: `(food_dist - neighbor_food_dist) / food_dist`  
- Positiv: Bewegung nähert ans Essen an  
- Negativ: Bewegung entfernt  
- Betrag wächst je näher das Essen ist (1.0 = Essen direkt daneben)  
- 0.0 bei Wand (kein Nachbar)

**B — Globale Nahrungsdistanz** (20. Feature):  
`food_dist / max_steps` ∈ [0, 1] — absolute Entfernung zum Essen, normalisiert.

---

## 2. Smoke-Run (Verifikation)

```
gen    0  best    130.00  mean     42.50
gen    1  best    475.80  mean     92.15
gen    2  best    421.50  mean    125.12
best fitness: 475.80
```

Vergleichbar mit Run 002 Smoke (476.55) — neues Feature-Format validiert.

---

## 3. Lernkurve

<!-- Wird nach Abschluss aus fitness.csv befüllt -->

| Generation | Best Fitness | Mean Fitness | Bemerkung |
|---|---|---|---|
| 0   | _todo_ | _todo_ | |
| 40  | 4707   | ~2900  | Früher Stand — besser als Run 002 (≈3000 gen 40) |
| 100 | _todo_ | _todo_ | |
| 500 | _todo_ | _todo_ | |
| 1000| _todo_ | _todo_ | |
| 1500| _todo_ | _todo_ | |
| 1999| _todo_ | _todo_ | |
| best| _todo_ | —      | |

---

## 4. Benchmark-Ergebnis

<!-- Nach Abschluss befüllen -->

```bash
cp training-out/run-003/best.mlp crates/snake-core/assets/neural-net-ga/best.mlp
cargo run --release -p snake-core --example benchmark 50 10000
```

### Vergleich Run 001 / 002 / 003

| Topologie | Run 001 (Walls-only) | Run 002 (Mixed) | Run 003 (A+B) | Δ zu 002 |
|---|---:|---:|---:|---:|
| Walls Ø  | 80.60 | 63.72 | _todo_ | _todo_ |
| Periodic Ø | 6.08 | 85.32 | _todo_ | _todo_ |

---

## 5. Checkpoint-Vergleich

<!-- Nach Abschluss befüllen -->

| Checkpoint | Ø Walls | Ø Periodic | Δ Walls zu 002 | Δ Periodic zu 002 |
|---|---:|---:|---:|---:|
| gen 100  | _todo_ | _todo_ | _todo_ | _todo_ |
| gen 500  | _todo_ | _todo_ | _todo_ | _todo_ |
| gen 1000 | _todo_ | _todo_ | _todo_ | _todo_ |
| gen 1500 | _todo_ | _todo_ | _todo_ | _todo_ |
| gen 1900 | _todo_ | _todo_ | _todo_ | _todo_ |
| best     | _todo_ | _todo_ | _todo_ | _todo_ |

---

## 6. Beobachtungen

### Warum A+B theoretisch helfen sollte

Das Binary-Flag in Run 001/002 verlor Distanzinformation. Das Netz wusste bei
`approaches_food = 1.0`, dass Essen irgendwo in Richtung liegt — aber ob es
1 oder 20 Felder entfernt ist, blieb verborgen. Das neue Feature kombiniert
beides: Richtung UND Annäherungsintensität.

Besonders auf Periodic ist das wichtig: Der Torus hat keine Wände als
Orientierungspunkte; das Netz muss stärker auf Essen-Distanz-Signale
angewiesen sein, um nicht in Schleifen zu laufen.

<!-- Nach Abschluss: Was hat sich tatsächlich verändert? -->

---

## 7. Fazit (nach Abschluss)

_Hat A+B den Walls-Score gegenüber Run 002 verbessert?_  
_Hat Periodic profitiert oder gelitten?_  
_Ist das kontinuierliche Feature ein klarer Gewinn?_

---

## Dateien

| Datei | Beschreibung |
|---|---|
| `crates/snake-core/assets/neural-net-ga/best.mlp` | Eingechecktes Netz |
| `training-out/run-003/fitness.csv` | Lernkurve (nicht eingecheckt) |
| `training-out/run-003/gen_*.mlp` | Checkpoints alle 100 Gen |
