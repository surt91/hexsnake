# Training: AlphaZero-light

## 1. Überblick

AlphaZero-light ist eine **Monte-Carlo-Baumsuche (MCTS)**, die statt
Zufalls-Rollouts (wie beim `MonteCarlo`-Lookahead) von einem **Policy/Value-
Netz** geführt wird: Ein MLP mit 7 Ausgaben liefert sechs Policy-Logits
(blickrichtungsrelative Richtungen) als PUCT-Priors und einen Value zur
Blattbewertung. Die Kantenwerte enthalten zusätzlich ein **dichtes
Schritt-Reward** (Futter-Annäherung + Fress-Bonus), damit die Suche schon mit
untrainiertem Value Futter ansteuert — sonst kollabiert die Policy zu „sicher
im Kreis fahren". Gewählt wird der meistbesuchte Zug.

> **AZ-eigener 21-Input** (`az_features`): die 20 geteilten Sensor-Features plus
> ein **Topologie-Bit** (1.0 = Walls, 0.0 = Torus). Optimal-Spiel unterscheidet
> sich je Rand; der Bit lässt *ein* Netz Policy/Value je Topologie konditionieren
> (Run 029). Die geteilte `features` (20) und die NEAT/DQN/PPO/MLP-GA-Netze
> bleiben unangetastet. Das deployte Netz ist **21→64→48→7** (`--hidden 64 48`).

Strategie: `snake_core::strategy::AlphaZeroLite` (pur Rust, WASM-fähig,
**Inferenz bleibt immer pur Rust/WASM**).

**Zwei Trainer:**
- **Gradient (primär, PyTorch)**: `python/train_alphazero.py`. Self-Play läuft
  komplett in Rust (`az_selfplay`, GIL freigegeben → alle Cores), mit *exakt
  derselben* Suche wie die Inferenz — keine zweite MCTS, die divergieren
  könnte. Python macht nur den Gradientenschritt: Policy-Loss =
  Cross-Entropy gegen die MCTS-Besuchsverteilung, Value-Loss = MSE gegen den
  (tanh-)Return.
- **Gradientenfrei (Fallback, ohne Python)**: `snake-train az` — evolviert das
  Netz per ES auf den Spielergebnissen. Für reine Rust-Umgebungen.

> **Such-Budget muss zusammenpassen**: Der Value-Kopf ist für die
> Trainings-Tiefe (`--sims`) kalibriert. `AlphaZeroLite::embedded()` spielt
> mit demselben Budget (aktuell **24**). Beim Einbetten eines mit anderem
> `--sims` trainierten Netzes den Wert in `alphazero.rs` angleichen.

> **Eingecheckte Datei**: `crates/snake-core/assets/alphazero/best.mlp` ist das
> Netz aus Run 029 (21→64→48→7, Seed 1) — Walls 53.0 / Periodic 75.5 / Avg 64.24
> (200 Spiele).

## 2. Gradient-Training (empfohlen)

Setup wie bei DQN/PPO (siehe `python/README.md`): `uv sync --extra train`.

### Smoke-Run (immer zuerst)

```bash
cd python
uv run --extra train python train_alphazero.py \
  --iterations 2 --games-per-iter 8 --sims 8 --max-ticks 300 --out /tmp/az-smoke.mlp
```

Beim Start läuft ein **Export-Self-Check** (numpy/Torch-Netz == Rust-Inferenz
über `mlp_forward`), der das 7-Output-Gewichtslayout absichert.

### Echter Lauf

```bash
uv run --extra train python train_alphazero.py \
  --iterations 300 \
  --games-per-iter 128 \
  --sims 24 \
  --temperature 1.0 \
  --max-ticks 1500 \
  --epochs 4 \
  --boundary mixed \
  --seed 1 \
  --hidden 64 48 --lr 5e-4 \
  --eval-every 5 --eval-games 20 --eval-max-ticks 4000 \
  --out az.mlp
```

- `--hidden 64 48` ist die aktuelle Architektur (21→64→48→7); mit dem
  Topologie-Bit braucht das Netz die Kapazität, um beide Topologien zu meistern
  (kleineres Netz spezialisiert sich nur — Run 028 vs. 029). LR 5e-4 für das
  größere Netz.
- `--max-hours N` begrenzt per Wall-Clock statt Iterationszahl; best.mlp wird
  laufend beim besten Eval gesichert. Hinweis: Das Training **plateaut früh**
  (~15–40 min) — sehr lange Läufe bringen wenig (Run 025/027/029).

- Self-Play parallelisiert über `--workers` (Default = alle Cores), da der
  GIL während der Rust-Suche freigegeben wird.
- `--sims` = Such-Budget pro Zug; merken, `embedded()` muss übereinstimmen.
- `--temperature` steuert die Exploration beim Ziehen aus den Besuchszahlen
  (1.0 = proportional, →0 = greedy).
- **Board-Vielfalt**: Jedes Self-Play-Spiel läuft auf einem eigenen Board-Seed
  (nicht mehr fix Seed 0). Das ist entscheidend gegen Überfitten/Kreisen.
- **Checkpoint-Auswahl per greedy Eval**: Alle `--eval-every` Iterationen
  spielt der Trainer `--eval-games` greedy Spiele je Topologie auf Board-Seeds
  `0..N` (wie `bench_mlp`) bei `--eval-max-ticks` und speichert `best.mlp` nach
  dem besten Walls+Periodic-Mittel. Das spiegelt das Deployment-Verhalten —
  Auswahl nach Self-Play-Score oder Überlebenszeit speichert Kreis-Kollaps.
  Die Log-Spalten `sp_score`/`sp_ticks` sind nur Self-Play-Diagnose.
- **Längeres Training hilft jetzt**: Mit Board-Vielfalt + greedy-Eval-Auswahl
  steigt die Qualität über 300 Iterationen weiter (kein „länger → schlechter"
  mehr; siehe Run 025).

## 3. Gradientenfreier Fallback (Rust, ohne Python)

```bash
cargo run --release -p snake-train -- az --smoke --out /tmp/az-smoke   # zuerst
cargo run --release -p snake-train -- az \
  --mixed --generations 400 --population 96 --games 6 \
  --max-ticks 2000 --sims 24 --seed 1 --out training-out/az-run-001
```

Fitness = ⌀(Score·100 + Ticks·0,1). MCTS macht jeden Tick teuer; rayon
parallelisiert über die Population.

## 4. Auswertung & Einbetten

```bash
cp az.mlp crates/snake-core/assets/alphazero/best.mlp   # bzw. best.mlp aus dem GA-Lauf
# embedded()-Sims in alphazero.rs an --sims angleichen, falls geändert.
cargo test -p snake-core alphazero
cargo run --release -p snake-core --example benchmark 30 5000   # enthält AlphaZero-light
```

Danach ist die Strategie im Dropdown „AlphaZero-light" aktiv (nativ + WASM).
Das Overlay (Taste `O`) zeigt die Besuchszahlen je Richtung.
