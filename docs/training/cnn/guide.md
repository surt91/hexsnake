# Training: Conv-Netz (ganzes Brett als Input)

## 1. Überblick

Das **Conv-Netz** sieht — anders als die Sensorstrahlen-Netze (MLP/NEAT/DQN/
PPO) — das **gesamte Spielfeld** als Gitter-Tensor. Die Inferenz ist eine
**reine Rust-Hex-Faltung** (`snake_core::nn::HexConv`), läuft also unverändert
nativ und in WASM — kein ONNX-Runtime, keine schwere ML-Crate.

Es gibt **zwei** Strategien auf demselben Conv-Stack:

- **Conv-Netz** (`ConvNet`, `.cnn`, 6 Ausgaben): eigenständige Policy, sechs
  **absolute** Richtungs-Scores; gewählt wird die beste sichere Richtung.
- **AlphaZero-Conv** (`AlphaZeroConv`, `.cnn`, 7 Ausgaben): dieselbe MCTS wie
  `AlphaZeroLite`, aber das Policy/Value-Netz ist das Conv-Netz. Die sechs
  absoluten Policy-Logits werden je Blickrichtung in den relativen Frame
  rotiert, damit die geteilte Suche (`run_search`) unverändert bleibt. Die
  bestehende MLP-Variante (`AlphaZero-light`) bleibt **daneben** erhalten.

### Architektur

- **Eingabe-Ebenen** (`CONV_CHANNELS = 4`, je `H×W`): Körper (ohne Kopf), Kopf,
  Futter, **Topologie** (konstante Ebene, 1.0 Walls / 0.0 Torus).
- **Hex-Faltung 7-tap**: Zentrum + 6 Nachbarn. Das Nachbar-Gathering nutzt
  `Board::neighbor()` — Wände → Zero-Pad, Torus → Wrap; deterministisch und
  torus-korrekt ohne Sonderfall.
- **Größenunabhängig ohne Positions-Blindheit**: nach dem Conv-Stack wird der
  Feature-Vektor **an der Kopfzelle** (lokaler Kontext) mit dem **globalen
  Average-Pool** (Brett-Bilanz) konkateniert und in einen Dense-Head (`Mlp`,
  tanh-hidden, linearer Output) gegeben. Beide Hälften sind unabhängig von
  Breite/Höhe → ein Netz spielt auf allen Presets und freien Größen.

> **Umgekehrte Entscheidung.** Früher galt „CNN-Policies nicht nach WASM
> einbetten" (`docs/training/dqn/guide.md` §6). Mit der handgeschriebenen
> Hex-Faltung ist die Inferenz nur unwesentlich mehr Code als `Mlp::forward`
> und bleibt pur Rust/WASM — die CNN-Netze werden jetzt eingebettet.

### Gewichtsformat `.cnn`

Textformat parallel zu `.mlp` (siehe `crates/snake-core/src/nn/conv.rs`):
Magic-Zeile, `channels N`, je Conv-Layer `conv in out`, `head <dims…>`,
`params`, dann die flachen `f32`-Parameter. Reihenfolge: pro Conv-Layer der
`out×in×7`-Kernel (row-major `[o][i][tap]`) dann die `out` Biases, danach die
`Mlp`-Head-Parameter.

## 2. Pipeline-Selbstcheck (immer zuerst)

Sichert das Python→Rust-Layout (Kernel-Reihenfolge, Readout⊕Pool, Head) gegen
die **echte** Rust-Inferenz ab — unabhängig vom Training:

```bash
cd python
uv run --extra train python verify_cnn_roundtrip.py
# -> max abs error torch vs rust: ~1e-7; OK
```

Die PyTorch-Referenz (`hexsnake_rl/hexconv.py`) spiegelt `HexConv` exakt und
bezieht die Nachbar-Geometrie über `neighbor_table` aus Rust — die Hex-/
Torus-Mathematik wird **nicht** in Python nachgebaut.

## 3. Conv-Netz trainieren (Behavior Cloning)

`train_cnn.py` kloniert den stärksten klassischen Autopiloten — den
A*-`PathPlanner` (+ Tail-Check) — in das `HexConvNet` und exportiert `.cnn`. Die
Experten-Zustände kommen aus dem Rust-Binding `expert_rollout` (pur Rust, GIL
frei) mit der **absoluten** Planner-Richtung als Label (das ConvNet gibt
absolute Scores aus → keine Blickrichtungs-Umrechnung). Beide Topologien werden
gemeinsam trainiert; jeder Sample läuft durch die Gather-Tabelle seiner
Topologie (Walls Zero-Pad / Torus Wrap).

Smoke-Run (Agent):

```bash
cd python
uv run --extra train python train_cnn.py --games 20 --epochs 5 \
  --out training-out/cnn/smoke.cnn
```

Echter Lauf (vom Nutzer; entspricht Run 001):

```bash
uv run --extra train python train_cnn.py --games 150 --epochs 40 \
  --toward-weight 4.0 --out training-out/cnn/best.cnn
```

> **`--toward-weight` gegen das Kreisen.** Reines BC kollabiert hier zu
> „sicherem Kreisen" (überlebt lange, frisst kaum). `--toward-weight N`
> gewichtet die Verlustfunktion auf futter-annähernden Zügen (Flag aus
> `expert_rollout`) N-fach. Half nur moderat — siehe
> [`run-001-report`](run-001-report/report.md); ein *starker* Lauf braucht
> RL/Self-Play, nicht BC.

## 4. AlphaZero-Conv trainieren

Self-Play wie bei AlphaZero-light, aber mit dem Conv-Netz als Evaluator
(Policy 6 + Value). Empfohlen analog `train_alphazero.py`: Self-Play in Rust,
Gradientenschritt in Python (Policy-CE gegen MCTS-Besuche, Value-MSE gegen den
tanh-Return), Export über `export_cnn`. Such-Budget (`--sims`) und
`AlphaZeroConv::embedded()`-Sims (aktuell **24**) müssen zusammenpassen, da der
Value-Kopf auf die Tiefe kalibriert ist.

## 5. Auswerten & Einbetten

```bash
# Vergleich gegen die Sensorstrahlen-Netze:
cargo run --release --example benchmark -- 50 10000

# Einbetten: trainierte Datei an die eingecheckte Stelle kopieren.
cp python/training-out/cnn/best.cnn crates/snake-core/assets/cnn/best.cnn
# bzw. crates/snake-core/assets/alphazero-cnn/best.cnn
cargo test -p snake-core   # embedded_*-Tests prüfen Laden + legales Spiel
```

> **Eingecheckte Dateien**:
> - `crates/snake-core/assets/cnn/best.cnn` = **Run 001** (BC, `--toward-weight 4`,
>   Walls 11.72 / Periodic 8.55, deployed — siehe
>   [`run-001-report`](run-001-report/report.md)). Funktionsfähig, aber
>   spielerisch schwach (überlebt lange, frisst wenig).
> - `…/alphazero-cnn/best.cnn` = deterministisches Smoke-Artefakt
>   (`python/gen_cnn_smoke.py`), bis ein echter Self-Play-Lauf es ersetzt.
