# Training Report: AlphaZero-light — Run 016

**Datum**: 2026-06-15  
**Ziel**: Breiterer Seed-Sweep (Seeds 8–18, ohne 12) mit bewährten Parametern.
Baseline: Run 015 Seed 5 (Walls Ø 36.66, Periodic Ø 60.17 — 200 Spiele).

---

## 1. Setup (alle 10 Seeds identisch außer `--seed`)

| Parameter | Wert |
|---|---|
| `--iterations` | 80 |
| `--boundary` | mixed |
| `--eat-bonus` | 0.3 |
| `--max-ticks` | 1 500 |
| `--lr` | 1e-3 |
| `--games-per-iter` | 128 |
| `--sims` | 24 |
| `--hidden` | 32 24 (Standard) |
| `--seed` | 8 / 9 / 10 / 11 / 13 / 14 / 15 / 16 / 17 / 18 |

10 parallele Läufe auf einem 32-Kern-Rechner (Systemlast ~220, ~7× überbuchte CPUs).
Trainingszeit daher ca. 40 Minuten statt normalerweise ~10 Minuten pro Lauf.

---

## 2. Benchmark-Ergebnis

Gemessen mit 50 Spielen je Checkpoint (best.mlp und final.mlp), sims=24, max_ticks=8000.
Seed 15 best.mlp zusätzlich mit 200 Spielen verifiziert.

### Alle Seeds (50 Spiele, best.mlp / final.mlp)

| Seed | Walls best | W final | Periodic best | P final | Avg best |
|-----:|-----------:|--------:|--------------:|--------:|--------:|
| Run 015 s5 (Baseline, 200 Sp.) | 36.66 | — | 60.17 | — | 48.42 |
| s8 | 36.18 | 34.76 | 53.12 | 56.40 | 44.65 |
| s9 | 24.62 | 21.96 | 56.62 | 56.84 | 40.62 |
| s10 | 35.46 | 36.56 | 57.64 | 56.74 | 46.55 |
| s11 | 31.00 | 31.00 | 49.66 | 49.66 | 40.33 |
| s13 | 31.46 | 33.04 | 55.60 | 55.90 | 43.53 |
| s14 | 38.50 | 36.08 | 52.88 | 56.98 | 45.69 |
| **s15** | **38.96** | **40.62** | **69.66** | 57.58 | **54.31** |
| s16 | 27.36 | 27.36 | 61.66 | 61.66 | 44.51 |
| s17 | 27.92 | 29.22 | 50.96 | 49.20 | 39.44 |
| s18 | 34.08 | 35.10 | 50.36 | 56.14 | 42.22 |

### Seed 15 Verifikation mit 200 Spielen

| Topologie | Run 015 s5 | **Seed 15 best** | Δ |
|-----------|----------:|----------------:|--:|
| Walls Ø | 36.66 | **37.26** | **+1.7 %** |
| Periodic Ø | 60.17 | **64.96** | **+7.9 %** |
| Avg Ø | 48.42 | **51.11** | **+5.6 %** |

**Seed 15 best.mlp schlägt Run 015 Seed 5 auf beiden Metriken.** Deployed.

---

## 3. Analyse

### 3.1 Sweet-Spot und Checkpoint-Verhalten

Mehrere Seeds zeigen: best.mlp ≠ bestes Benchmark-Ergebnis.

- **Seed 15**: best.mlp (Periodic 69.66) vs. final.mlp (Periodic 57.58)  
  → best.mlp wurde gespeichert, als game_len in der Selbstspiel-Phase peak hatte.
  → Dieses Netz hatte noch nicht die Walls-Effizienz der final.mlp, aber exzellentes Periodic-Verhalten.
- **Seed 14**: best.mlp Walls 38.50 (besser), final.mlp Periodic 56.98 (besser)
  → Ähnliches Muster: frühe Checkpoint-Verbesserungen bei einer Metrik.

Fazit: Der Trainings-Peak (höchste Selbstspiel-game_len) korreliert nicht perfekt mit
dem Benchmark-Peak. In der Praxis ist der beste Checkpoint oft weder der erste noch der letzte.

### 3.2 Seed-Spezialisierung

Run 016 zeigt deutlich ausgeprägtere Spezialisierungen als Run 015:
- **Walls-Spezialisten** (s14, s15 final): Walls >38, Periodic <60
- **Periodic-Spezialisten** (s15 best, s16): Periodic >60, Walls <32
- **Balanced** (s10, s18): keine klare Dominanz auf einer Metrik

Seed 15 best.mlp ist ein seltener Fund: balanced UND stark (Walls 37+, Periodic 65+).

### 3.3 Beste Gesamtergebnis

Seed 15 best.mlp mit 200 Spielen: Walls 37.26, Periodic 64.96, Avg 51.11.
Das ist das beste je für dieses Projekt gemessene kombinierte Ergebnis.

---

## 4. Schlussfolgerungen

**Seed 15 best.mlp deployed** — übertrifft Run 015 Seed 5 auf beiden Metriken
(+1.7% Walls, +7.9% Periodic über 200 Spiele).

Nächste Schritte:
- Run 017 (großes 128-96-Netz): läuft parallel, Ergebnis ausstehend
- Run 018: abhängig von Run 017-Ergebnis
  - Wenn 128-96 besser: Seed-Sweep mit großem Netz
  - Wenn Standard besser: weitere Seeds (20–30) oder LR-Verringerung

---

## 5. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-016-s15/best.mlp` | **Seed 15 best (Walls 37.26, Periodic 64.96) — deployed!** |
| `training-out/az-run-016-s{8..18}/best.mlp` | Beste Checkpoints je Seed |
| `training-out/az-run-016-s{8..18}/final.mlp` | Finale Checkpoints je Seed |
| `crates/snake-core/assets/alphazero/best.mlp` | **Run 016 Seed 15 — deployed** |
