# Training: Neural Net (GA/ES)

## 1. Überblick

Trainiert wird das Mini-MLP (19 Sensor-Features → 16 → 12 → 6
Richtungs-Scores, `snake_core::nn`) mit einer Evolutionsstrategie:
Population aus flachen Gewichtsvektoren, Truncation-Selektion +
Gauß-Mutation, Fitness = ⌀(Score·100 + Ticks·0,1) über mehrere Partien.
Trainer: `crates/snake-train` (rein nativ, rayon-parallel über die
Population).

Ergebnis ist eine Gewichtsdatei im dokumentierten Textformat
(`hexsnake-mlp v1`, siehe `crates/snake-core/src/nn/mlp.rs`):

- `<out-dir>/best.mlp` — bestes Netz des Laufs (laufend aktualisiert)
- `<out-dir>/gen_XXXXX.mlp` — Zwischen-Checkpoints
- `<out-dir>/fitness.csv` — Lernkurve (`generation,best,mean`)

Ins Spiel kommt das Netz über die eingecheckte Datei
`crates/snake-core/assets/mlp-ga/best.mlp` (per `include_str!`
eingebettet, Dropdown-Eintrag „Neural Net"). Die aktuell eingecheckte
Datei stammt aus einem **Smoke-Run** und ist nur Platzhalter-Qualität.

## 2. Voraussetzungen

- Rust-Toolchain (stable reicht): <https://rustup.rs>
- Repo-Checkout; alle Befehle laufen vom Repo-Root.
- Hardware: nur CPU, kein GPU nötig. Der Trainer skaliert über
  rayon mit den Kernen (Population parallel); RAM-Bedarf ist minimal
  (<100 MB). Mehr Kerne ⇒ proportional schnellere Generationen.
- Threads begrenzen (optional): `RAYON_NUM_THREADS=8`.

## 3. Smoke-Run (immer zuerst)

```bash
cargo run --release -p snake-train -- --smoke --out /tmp/hexsnake-smoke
```

Erwartete Ausgabe (Sekunden): drei Zeilen `gen 0..2` mit endlicher
Fitness, dann `best fitness: … -> /tmp/hexsnake-smoke/best.mlp`. Damit
sind Build, Evaluation, Checkpoints und Format verifiziert.

## 4. Echter Lauf

Empfohlener Startpunkt (Preset Klein, Wände — das Standard-Setup der
Fitness):

```bash
cargo run --release -p snake-train -- \
  --generations 2000 \
  --population 256 \
  --games 12 \
  --max-ticks 3000 \
  --sigma 0.08 \
  --seed 1 \
  --checkpoint-every 100 \
  --out training-out/run-001
```

- **Laufzeit**: grob 1–3 h auf einem 16-Kern-Desktop (skaliert linear
  mit `generations × population × games × max-ticks / Kerne`). Der
  Smoke-Run liefert einen Hochrechnungs-Anker: seine Laufzeit ×
  `(2000·256·12·3000)/(3·12·2·300)` ÷ (Kerne-Verhältnis).
- **Logs/Checkpoints**: siehe Überblick; `fitness.csv` eignet sich
  direkt für gnuplot/Pandas.
- **Abbruch/Fortsetzen**: `best.mlp` ist nach jedem Verbesserungsschritt
  konsistent auf Platte — Abbrechen (Ctrl-C) verliert nichts außer
  Fortschritt. Ein Resume aus Checkpoints ist **nicht** implementiert;
  ein neuer Lauf startet frisch (anderen `--seed` wählen und Läufe in
  getrennte `--out`-Verzeichnisse legen).
- **Reproduzierbarkeit**: gleiche Parameter + gleicher `--seed` ⇒
  identischer Lauf (deterministische Spiel-Engine, ein RNG im Trainer).
  Achtung: Die Parallelisierung ändert nichts am Ergebnis, nur an der
  Dauer.

## 5. Hyperparameter

| Parameter | Default | Wirkung | Sinnvoller Bereich |
|---|---|---|---|
| `--generations` | 300 | Trainingsdauer; mehr = besser bis zur Sättigung | 500–5000 |
| `--population` | 96 | Explorationsbreite pro Generation | 64–512 |
| `--games` | 6 | Partien pro Fitness-Messung (Rauschunterdrückung) | 6–24 |
| `--max-ticks` | 2000 | Tick-Deckel pro Partie (gegen Endlos-Stalling) | 1500–5000 |
| `--sigma` | 0.08 | Mutationsstärke | 0.02–0.2 |
| `--checkpoint-every` | 25 | Abstand der Zwischen-Checkpoints | 25–200 |
| `--seed` | 1 | Reproduzierbarkeit | beliebig |

Tuning-Hinweise:

- **Fitness stagniert früh** (best ≈ mean): zuerst `--sigma` erhöhen
  (0.12–0.2) oder `--population` verdoppeln — typisch zu wenig
  Exploration.
- **Fitness springt stark, mean bleibt schlecht**: `--games` erhöhen —
  die Messung ist zu verrauscht, Glückstreffer dominieren die Selektion.
- **Best fällt zwischen Generationen sichtbar ab**: nicht möglich
  (Eliten überleben unverändert); fällt die *Kurve* in `fitness.csv`
  trotzdem, vergleichst du verschiedene Seeds/Boards — Lauf-Setups nicht
  mischen.

## 6. Auswertung

Benchmark gegen die klassischen Strategien (das eingebettete Netz tritt
als „Neural Net" an):

```bash
cp <out-dir>/best.mlp crates/snake-core/assets/mlp-ga/best.mlp
cargo run --release -p snake-core --example benchmark 50 10000
```

Zielwerte (16×12): Das Phasenkriterium ist **⌀-Score deutlich über
Greedy** (Greedy ≈ 21–23 auf Walls). Zur Einordnung: Der Smoke-Run
(120 Generationen, Population 48) erreichte bereits ⌀ 37 auf Walls;
ein echter Lauf sollte klar darüber liegen. Monte-Carlo (≈ 62) ist die
nächste Messlatte.

Checkpoints vergleichen: jeweils nach
`crates/snake-core/assets/mlp-ga/best.mlp` kopieren und den
Benchmark erneut laufen lassen (oder mehrere `gen_*.mlp` nacheinander
durchmessen) — die Lernkurve über Generationen ist später auch ein
hübsches Blog-Diagramm (`fitness.csv` aufheben!).

## 7. Ergebnis einchecken

1. `cp <out-dir>/best.mlp crates/snake-core/assets/mlp-ga/best.mlp`
2. Optional 2–3 markante Zwischenstände (z. B. `gen_00100.mlp`) daneben
   legen, falls später „Gen X"-Varianten im Dropdown gewünscht sind.
3. Prüfen: `cargo test --workspace` (validiert u. a., dass die Datei
   parst und zur Architektur passt) und der Benchmark aus Schritt 6.
4. Commit inkl. Benchmark-Zahlen in der Commit-Message; `fitness.csv`
   nicht einchecken, aber für Blog-Notizen sichern (`/blog-notes`).

Architektur ändern (HIDDEN-Layer in `snake-core/src/nn/mod.rs`) macht
alte Gewichtsdateien inkompatibel — dann Smoke-Run wiederholen und neue
Platzhalter-Gewichte einchecken.
