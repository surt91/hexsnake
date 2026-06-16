# Training Report: AlphaZero-light — Run 028 (Topologie-Bit)

**Datum**: 2026-06-16
**Ziel**: Ein **Topologie-Feature** (1.0 = Walls, 0.0 = Torus) zum AZ-Input
hinzufügen. Hypothese: Run 027 zeigte, dass ein einzelnes Netz nicht beide
Topologien dominiert (finaler Checkpoint = Walls-Spezialist). Optimal-Spiel
unterscheidet sich je Rand; der explizite Bit soll dem Netz erlauben, Policy und
Value je Topologie zu konditionieren.

Referenz (deployed): Run 027 Seed 1 — Walls 45.73, Periodic 78.44, Avg 62.09.

---

## 1. Setup

AZ-eigener 21-Input (`az_features` = 20 Sensoren + Topologie-Bit). Netz 21→32→24→7
(Standard). Sonst wie Run 027: Seed 1, `--max-hours 1` (→ 3 298 Iterationen),
games-per-iter 128, sims 24, greedy-Eval-Auswahl.

## 2. Benchmark (`bench_mlp`, 200 Spiele, 8000 Ticks, sims 24)

| Netz | Walls | Periodic | Avg |
|---|---:|---:|---:|
| Run 027 (deployed) | 45.73 | **78.44** | **62.09** |
| Run 028 best (iter 610) | **50.50** | 63.01 | 56.76 |
| Run 028 final (iter 3297) | 50.49 | 62.88 | 56.69 |

## 3. Analyse — Bit allein hilft nicht

- **Netto −8.6 %**: +10 % Walls, aber −20 % Periodic. Das Bit verschob die
  Balance Richtung Walls, statt beide Topologien gleich gut zu machen — das Netz
  (Standardgröße) **spezialisierte sich** mit der neuen Information, statt zwei
  starke Policies zu halten.
- best und final fast identisch (W 50 / P 63) → es ist die gelernte Policy, kein
  Auswahl-Rauschen.
- **Schlussfolgerung**: Information ohne Kapazität reicht nicht. → Run 029 testet
  Topologie-Bit **plus größeres Netz**.

## 4. Dateien

| Datei | Beschreibung |
|---|---|
| `training-out/az-run-028-s1/best.mlp` | iter 610, 21-Input 32→24 (W 50.50 / P 63.01) |
| `training-out/az-run-028-s1/final.mlp` | iter 3297 |
