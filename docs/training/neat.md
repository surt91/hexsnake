# Training: NEAT

## 1. Überblick

NEAT (NeuroEvolution of Augmenting Topologies) entwickelt nicht nur die
Gewichte, sondern die **Netzstruktur** mit: Startpunkt ist ein minimales
Netz (20 Sensor-Features + Bias → 6 Richtungs-Scores, voll verbunden),
das durch Mutation neue Knoten und Verbindungen wachsen lässt. Dieselben
Sensor-Features und dieselbe Maskierung tödlicher Züge wie beim MLP
(`snake_core::nn`).

Trainer: `crates/snake-train` (rein nativ, rayon-parallel), Aufruf über das
Unterkommando `neat`. Bausteine: Innovation-Tracking (gleiche
Strukturänderung ⇒ gleiche Nummer), die drei Mutationen
(Gewicht / Verbindung hinzufügen / Knoten einfügen), Crossover entlang der
Innovationsnummern und **Speziation mit Fitness-Sharing**. Fitness =
⌀(Score·100 + Ticks·0,1) über mehrere Partien — identisch zum GA/ES-Trainer,
also direkt vergleichbar.

Ergebnis ist ein Genom im dokumentierten Textformat (`hexsnake-neat v1`,
siehe `crates/snake-core/src/nn/neat.rs`):

- `<out-dir>/best.neat` — bestes Genom des Laufs (laufend aktualisiert)
- `<out-dir>/gen_XXXXX.neat` — Zwischen-Checkpoints
- `<out-dir>/fitness.csv` — Lernkurve (`generation,best,mean,species`)

Ins Spiel kommt das Netz über die eingecheckte Datei
`crates/snake-core/assets/neat/best.neat` (per `include_str!` eingebettet,
Dropdown-Eintrag „NEAT"). Die aktuell eingecheckte Datei stammt aus einem
**Smoke-Run** (mixed-boundary, ~150 Generationen) und ist nur
Platzhalter-Qualität.

## 2. Voraussetzungen

- Rust-Toolchain (stable reicht): <https://rustup.rs>
- Repo-Checkout; alle Befehle laufen vom Repo-Root.
- Hardware: nur CPU, kein GPU. Der Trainer skaliert über rayon mit den
  Kernen (Population parallel), RAM-Bedarf minimal (<100 MB).
- Threads begrenzen (optional): `RAYON_NUM_THREADS=8`.

## 3. Smoke-Run (immer zuerst)

```bash
cargo run --release -p snake-train -- neat --smoke --out /tmp/hexsnake-neat
```

Erwartete Ausgabe (Sekunden): drei Zeilen `gen 0..2` mit endlicher Fitness
und Spezies-/Knotenzahl, dann `best fitness: … -> …/best.neat`. Damit sind
Build, Evaluation, Mutationen, Checkpoints und Format verifiziert.

## 4. Echter Lauf

Empfohlener Startpunkt (gemischte Ränder, damit das Netz Wände **und** Torus
beherrscht):

```bash
cargo run --release -p snake-train -- neat \
  --mixed \
  --generations 2000 \
  --population 300 \
  --games 12 \
  --max-ticks 3000 \
  --seed 1 \
  --compat 3.0 \
  --add-conn 0.06 \
  --add-node 0.03 \
  --checkpoint-every 100 \
  --out training-out/neat-run-001
```

- **Laufzeit**: grob 1–4 h auf einem 16-Kern-Desktop (skaliert linear mit
  Generationen × Population × Games). Längere Partien (`--max-ticks`) und
  mehr `--games` glätten die Fitness, kosten aber proportional Zeit.
- **Determinismus**: derselbe `--seed` reproduziert den Lauf exakt
  (seedbarer `Pcg64`, deterministisches `snake-core`).

### Wichtige Parameter

| Flag | Default | Wirkung |
|---|---|---|
| `--generations` | 300 | Anzahl Generationen |
| `--population` | 150 | Genome pro Generation |
| `--games` | 6 | Partien je Genom (Fitness-Mittel) |
| `--max-ticks` | 2000 | Tick-Limit pro Partie |
| `--mixed` | aus | je zur Hälfte Wände/Torus auswerten |
| `--compat` | 3.0 | Speziations-Schwelle (kleiner ⇒ mehr Spezies) |
| `--add-conn` | 0.06 | Rate „Verbindung hinzufügen" |
| `--add-node` | 0.03 | Rate „Knoten einfügen" |
| `--seed` | 1 | RNG-Seed (Reproduzierbarkeit) |

> **Tuning-Hinweis Speziation**: Bleibt die Spalte `species` in `fitness.csv`
> dauerhaft bei 1, ist `--compat` zu hoch — kleiner stellen (z. B. 1.5–2.0),
> damit verschiedene Topologien als eigene Nische geschützt werden und nicht
> sofort vom Champion verdrängt werden.

## 5. Auswertung

- `fitness.csv` plotten (`best`/`mean` über `generation`); ein gesunder Lauf
  steigt zunächst steil, dann flacher. `species > 1` zeigt, dass die
  Speziation greift.
- Stärke headless vergleichen — der Benchmark enthält NEAT:

  ```bash
  cargo run --release -p snake-core --example benchmark 50 8000
  ```

  Erwartung: NEAT schlägt Greedy deutlich; nach einem langen Lauf nähert es
  sich Monte-Carlo/Raumgreifer. (Der Smoke-Run liegt nur knapp über Greedy.)

## 6. Einbetten

Das beste Genom als Asset übernehmen und einchecken:

```bash
cp training-out/neat-run-001/best.neat crates/snake-core/assets/neat/best.neat
cargo test -p snake-core embedded_neat   # parst & spielt legal?
cargo run --release -p snake-core --example benchmark 30 5000
```

Danach ist das neue Netz im Dropdown „NEAT" aktiv (nativ und im
WASM-Build). Asset-Wechsel zusammen mit dem zugehörigen Commit pflegen.
