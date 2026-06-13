# Training: AlphaZero-light

## 1. Überblick

AlphaZero-light ist ein **Monte-Carlo-Baumsuche (MCTS)**, die statt
Zufalls-Rollouts (wie beim `MonteCarlo`-Lookahead) von einem **Policy/Value-
Netz** geführt wird: Ein MLP mit 7 Ausgaben liefert sechs Policy-Logits
(blickrichtungsrelative Richtungen) als PUCT-Priors und einen Value, mit dem
Blattknoten bewertet werden. Gewählt wird der meistbesuchte Zug.

Strategie: `snake_core::strategy::AlphaZeroLite` (pur Rust, WASM-fähig).
Trainer: `snake-train az` — das Netz wird **gradientenfrei per ES** (wie GA/ES
und NEAT) auf den Spielergebnissen der MCTS-geführten Partien evolviert. „Self-
Play" heißt hier: Das Netz steuert seine eigene Vorausschau, und wir
selektieren es nach dem erreichten Score.

> **Wichtig — Such-Budget muss zusammenpassen**: Der Value-Kopf ist nur für
> die Tiefe kalibriert, mit der trainiert wurde (`--sims`). `AlphaZeroLite::
> embedded()` spielt deshalb mit demselben Budget (16). Mehr Sims ⇒ das Netz
> vertraut dem Value zu tief und beginnt, *sicher zu kreisen statt zu fressen*.
> Beim Einbetten eines neuen Netzes ggf. `embedded()`-Sims an `--sims`
> anpassen.

> **Eingecheckte Datei**: `crates/snake-core/assets/alphazero/best.mlp` ist
> ein **Smoke-Artefakt** (kurzer Lauf), Platzhalter-Qualität.

## 2. Voraussetzungen

- Rust-Toolchain (stable), Repo-Checkout. Nur CPU, rayon-parallel.
- Reward/Fitness wie bei den anderen ES-Trainern: ⌀(Score·100 + Ticks·0,1).

## 3. Smoke-Run (immer zuerst)

```bash
cargo run --release -p snake-train -- az --smoke --out /tmp/az-smoke
```

Drei Generationen mit endlicher Fitness, dann `best fitness: … -> …/best.mlp`.

## 4. Echter Lauf

```bash
cargo run --release -p snake-train -- az \
  --mixed \
  --generations 400 \
  --population 96 \
  --games 6 \
  --max-ticks 2000 \
  --sims 24 \
  --seed 1 \
  --checkpoint-every 50 \
  --out training-out/az-run-001
```

- `--sims` ist das **Such-Budget pro Zug**. Größer = stärkere Vorausschau,
  aber jeder Zug kostet entsprechend mehr; klein halten (16–32). Merke dir den
  Wert — `embedded()` muss damit übereinstimmen.
- **Laufzeit**: MCTS macht jeden Tick teuer; deutlich langsamer als der reine
  MLP-Trainer. Mehrere Stunden für einen großen Lauf; rayon skaliert mit den
  Kernen.

## 5. Auswertung & Einbetten

```bash
cp training-out/az-run-001/best.mlp crates/snake-core/assets/alphazero/best.mlp
# embedded()-Sims in alphazero.rs an --sims angleichen, falls geändert.
cargo test -p snake-core alphazero
cargo run --release -p snake-core --example benchmark 30 5000   # enthält AlphaZero-light
```

Danach ist die Strategie im Dropdown „AlphaZero-light" aktiv (nativ + WASM).
Das Overlay (Taste `O`) zeigt die Besuchszahlen je Richtung als
MCTS-Bewertung.
