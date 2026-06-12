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
- Gemessen werden je Strategie: ⌀-Score, Median, Max, ⌀-Überlebenszeit
  (Ticks), Timeout-Quote (Partie ohne Tod abgebrochen nach Tick-Limit).
- Beide Randbedingungen (Wände und periodisch) auf dem Mittel-Preset 24×18
  durchlaufen, Ausgabe als Markdown-Tabelle auf stdout.

## Erwartete Hackordnung (Plausibilitätscheck)

Chaos-Walker < Greedy < Raumgreifer ≈ Monte-Carlo < Pfadplaner; Hamilton hat
die höchste Überlebenszeit, aber langsame Scores. Weicht ein Ergebnis stark
davon ab, zuerst nach einem Bug suchen (häufig: Torus-Distanz im
periodischen Modus vergessen), bevor die Strategie „besser getunt" wird.
