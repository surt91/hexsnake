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

> **Eingecheckte Datei**: `crates/snake-core/assets/alphazero/best.mlp` ist
> ein kurzer Gradienten-Smoke-Lauf (mixed, ~40 Iterationen) — funktionsfähig,
> aber Smoke-Qualität.

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
  --iterations 60 \
  --games-per-iter 128 \
  --sims 24 \
  --temperature 1.0 \
  --max-ticks 1500 \
  --epochs 4 \
  --boundary mixed \
  --seed 1 \
  --out az.mlp
```

- Self-Play parallelisiert über `--workers` (Default = alle Cores), da der
  GIL während der Rust-Suche freigegeben wird.
- `--sims` = Such-Budget pro Zug; merken, `embedded()` muss übereinstimmen.
- `--temperature` steuert die Exploration beim Ziehen aus den Besuchszahlen
  (1.0 = proportional, →0 = greedy).

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
