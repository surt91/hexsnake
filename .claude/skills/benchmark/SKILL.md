---
name: benchmark
description: Headless-Benchmark der Autopilot-Strategien ausführen und Ergebnisse vergleichen (ab Phase 4). Nutzen, wenn Strategien bewertet, verglichen oder nach Änderungen auf Regressionen geprüft werden sollen.
---

# Strategie-Benchmark

Der Benchmark-Harness lebt in `snake-core` als Example-Binary und spielt pro
Strategie N headless-Partien mit festen Seeds:

```bash
cargo run --release -p snake-core --example benchmark
```

(`--release` ist wichtig — Debug-Builds verfälschen vor allem
Monte-Carlo-Ergebnisse, weil dessen Tick-Budget zeitbasiert sein kann.)

## Konventionen für den Harness

- Feste Seed-Liste (z. B. Seeds 0..100), damit Läufe vergleichbar und
  reproduzierbar sind — Strategien sehen identische Futter-Sequenzen.
- Gemessen werden je Strategie: ⌀-Score, ⌀-Überlebenszeit (Ticks),
  Max-Score, **`won%`** (Anteil Partien mit Status `Won`, Brett voll) und
  **`⌀ticks(won)`** (mittlere Ticks der gewonnenen Partien, `—` wenn keine).
- Perfect-Play-Referenzlauf:
  `cargo run --release -p snake-core --example benchmark -- 50 20000`.
  Tick-Limit 20 000, damit Walls-Hamilton-Partien nicht am Limit
  abgeschnitten werden; HamiltonRider muss auf dem Torus 100 % `won`
  zeigen (validiert die Metrik).
- Eine einzelne `.cnn`-Datei bencht `examples/bench_cnn.rs`
  (`<datei.cnn> <spiele> <max_ticks>`): 6-Output ⇒ `ConvNet`-Argmax,
  7-Output ⇒ `AlphaZeroConv`-MCTS. Analog `bench_mlp` für `.mlp`.
- Beide Randbedingungen (Wände und periodisch) auf dem Mittel-Preset 24×18
  durchlaufen, Ausgabe als Markdown-Tabelle auf stdout.

## Erwartete Hackordnung (Plausibilitätscheck)

Chaos-Walker < Greedy < Raumgreifer ≈ Monte-Carlo < Pfadplaner; Hamilton hat
die höchste Überlebenszeit, aber langsame Scores. Weicht ein Ergebnis stark
davon ab, zuerst nach einem Bug suchen (häufig: Torus-Distanz im
periodischen Modus vergessen), bevor die Strategie „besser getunt" wird.
