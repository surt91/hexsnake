# Plan 02 — CNN-Strategie (ganzes Brett als Input)

Ergänzt [`plan/01_snake.md`](01_snake.md) und [`docs/concept.md`](../docs/concept.md)
§3.8. Ziel: eine **convolutionale Strategie**, die das gesamte Spielfeld als
Gitter-Tensor sieht (statt der sechs Sensorstrahlen), mit **purer
Rust/WASM-Inferenz** — kein ONNX-Runtime, keine schwergewichtige ML-Crate.

Es entstehen **zwei** spielbare Strategien:

1. **ConvNet** — eigenständige Policy (6 Richtungs-Scores), analog zu
   `NeuralNet`/`NeatNet`.
2. **AlphaZero-CNN** — dieselbe MCTS-Suche wie `AlphaZeroLite`, aber das
   Policy/Value-Netz ist ein Conv-Netz. Die bestehende MLP-Variante
   (`AlphaZero-light`) **bleibt unverändert daneben bestehen**.

> **Bewusste Kursänderung.** `docs/training/dqn/guide.md` §6 hielt fest, dass
> CNN-Policies *nicht* nach WASM eingebettet werden („pure-Rust-Conv-Inferenz
> außerhalb des Scopes"). Dieser Plan kehrt diese Entscheidung absichtlich um.
> Die betroffenen Docs werden in Phase C.5 angepasst.

## Architektur-Entscheidungen

### Hex-Faltung statt Quadrat-Kernel
Ein klassischer 3×3-Kernel passt nicht aufs Hexgitter. Der natürliche Kernel
ist **7-tap: Zentrum + 6 Nachbarn**. Das Gathering nutzt `Board::neighbor()`,
das Wände (off-board → Null/Padding) und periodischen Rand (Wrap) bereits
korrekt unterscheidet — damit bleibt die Faltung deterministisch und
torus-korrekt, ohne Sonderfälle. Off-board-Nachbarn tragen 0 bei (Zero-Pad).

Kernel-Tap-Reihenfolge fix: Index 0 = Zentrum, 1..6 = `Direction::ALL` in
fester Reihenfolge (absolut, **nicht** heading-relativ).

### Größenunabhängigkeit ohne Positions-Blindheit
Globales Average-Pooling allein macht das Netz größenagnostisch, aber
**positionsblind** (es kollabiert das Brett auf Kanalmittelwerte). Lösung:
Nach dem Conv-Stack wird ein größenunabhängiger Vektor gebildet aus

- **Head-Cell-Readout**: der Feature-Vektor an der Kopfzelle (lokaler Kontext),
- **Global Average Pool**: Kanalmittel über alle Zellen (Bilanz: Freiraum,
  grobe Futter-/Körperverteilung),

die **konkateniert** in einen Dense-Head (`Mlp`) gehen. Beides ist
unabhängig von Breite/Höhe → ein Netz spielt auf allen Presets und freien
Größen.

### Eingabekanäle (H×W-Ebenen, absolut)
- `body` — Körper ohne Kopf (1.0 belegt),
- `head` — Kopfzelle (1.0),
- `food` — Futterzelle (1.0),
- `topology` — konstante Ebene (1.0 Wände / 0.0 Torus), analog zum AZ-Bit aus
  `az_features`.

Spiegelt die bestehende Python-`grid`-Observation
(`crates/snake-py/src/env.rs`, 3 Kanäle Körper/Kopf/Futter) und erweitert sie
um die Topologie-Ebene.

### Absolute vs. heading-relative Ausgaben
Sensor-Netze sind heading-relativ (Index 0 = geradeaus, rotationsinvariant);
ein Grid-CNN ist absolut (Nord ist Nord). Konsequenz:

- **ConvNet** gibt 6 **absolute** Richtungs-Scores aus; gewählt wird die beste
  *sichere* absolute Richtung (Masking wie bei den anderen Netzen).
- **AlphaZero-CNN** rotiert die 6 absoluten Policy-Logits per Heading in den
  relativen Frame, damit die MCTS-Suche (`rotated_cw`, REVERSE-Index)
  **unverändert** bleibt. Value-Head wie gehabt (1 Output, `tanh`).

### Gewichtsformat `.cnn`
Neues Textformat parallel zu `.mlp` (gleiche Philosophie: Magic + Shapes +
flache `f32`-Parameter, `{:?}`-Roundtrip). Genug, um Conv-Kernel + Biases je
Layer plus den `Mlp`-Head zu speichern. Skizze:

```text
hexsnake-cnn v1
in_channels=4
conv 4 8        <- conv-Layer: in_ch out_ch (Kernel implizit 7-tap)
conv 8 8
head 16 12 6    <- Dense-Head-Dims (readout+pool konkateniert → … → out)
<flache Parameter, whitespace-getrennt, 16 pro Zeile>
```

Conv-Hidden nutzen `tanh` (konsistent mit `Mlp`); der Head endet linear.
Head-Input-Dim = `2 * last_conv_out_channels` (Readout ⊕ Pool).

---

## Phase A — Core-Inferenz (`snake-core`)

- [x] **`crates/snake-core/src/nn/conv.rs`**: `HexConv`-Struct
      (`conv_layers: Vec<ConvLayer>`, `head: Mlp`, `in_channels`). `ConvLayer`
      hält `out_ch × in_ch × 7`-Kernel + `out_ch`-Biases als flachen `f32`-Vec.
- [x] **Plane-Builder**: aus `&GameState` die `in_channels × H × W`-Ebenen
      bauen (body/head/food/topology), kanal-major dann row-major — Layout wie
      `Env::grid_observation`.
- [x] **Forward**: pro Conv-Layer 7-tap-Gather via `board.neighbor()`
      (off-board → 0), `tanh`; danach Head-Cell-Readout ⊕ Global-Avg-Pool →
      `head.forward()`. Deterministisch, keine HashMap-Iteration, WASM-fähig.
- [x] **`.cnn`-Format**: `to_text`/`from_text` + `param_count`, analog
      `Mlp`. Tests: exakter Roundtrip, Ablehnung von Garbage, Shape-Check.
- [x] **Tests**: bekannte Kleinwerte (1 Kanal, 1 Conv-Layer, fester Kernel),
      Wrap-vs-Wall-Verhalten der Faltung am Rand, Größenunabhängigkeit
      (gleiches Netz auf 16×12 und 24×18 lauffähig), Determinismus.

**Done wenn:** `HexConv` lädt aus `.cnn`-Text, liefert für einen `GameState`
einen deterministischen Output-Vektor, und alle Conv-Tests sind grün.

## Phase B — Strategien (`snake-core`)

- [x] **`ConvNet`-Strategie**: `Strategy`-Impl, 6 absolute Scores → sichere
      beste Richtung; `StrategyDebug.move_scores` füllen. `embedded()` lädt ein
      eingebettetes Smoke-Asset `assets/cnn/best.cnn`.
- [x] **AlphaZero refaktorieren**: die Suche in `alphazero.rs` über einen
      Evaluator-Trait/Closure generisch machen
      (`fn evaluate(&GameState) -> ([f32;6], f32)`), damit `AlphaZeroLite`
      (MLP) **bitidentisch** bleibt und der Conv-Evaluator dieselbe Suche
      teilt. Self-Play/`AzSample` ebenfalls über den Evaluator.
- [x] **`AlphaZeroConv`-Strategie**: Conv-Evaluator (6 Policy-Logits in den
      relativen Frame rotiert + Value), `embedded()` lädt
      `assets/alphazero-cnn/best.cnn`.
- [x] **Tests**: beide Strategien spielen legal (kein tödlicher Zug bei
      sicherer Alternative), deterministisch; AZ-MLP-Tests bleiben unverändert
      grün (Regress-Schutz für das Refactoring).

**Done wenn:** `ConvNet` und `AlphaZeroConv` spielen headless legal und
deterministisch; die bestehende `AlphaZeroLite` ist unverändert.

## Phase C — Anbindung & Training

- [x] **C.1 Dropdown**: `StrategyChoice::ConvNet` und
      `StrategyChoice::AlphaZeroConv` in `settings.rs` (`ALL`, `label`,
      `compatible_with`) und `game_view.rs` (Konstruktion) ergänzen.
- [x] **C.2 Benchmark**: beide in den Benchmark-Harness aufnehmen
      (`examples/benchmark.rs`), Vergleichstabelle Sensorstrahlen vs. CNN.
- [x] **C.3 Python-Training**: Hex-Conv-Netz in PyTorch auf der bestehenden
      `HexSnakeGym(observation="grid")`-Observation (Topologie-Ebene ergänzen).
      Für ConvNet supervised/RL, für AlphaZero-CNN über `self_play`. Nur
      **Smoke-Run** seitens Agent; echte Läufe macht der Nutzer.
- [x] **C.4 Export**: Exporter schreibt das `.cnn`-Format (Kernel-Layout exakt
      wie der Rust-Reader erwartet); Roundtrip-Test Python→Rust.
- [x] **C.5 Docs**: `docs/training/cnn/guide.md` (Skill `/training-docs`);
      `docs/training/dqn/guide.md` §6 und `plan/01_snake.md` an die umgekehrte
      Embedding-Entscheidung anpassen; `docs/concept.md` §3.8 ergänzen.
- [x] **C.6 Assets**: eingebettete Smoke-Run-`.cnn`-Dateien für beide
      Strategien einchecken (werden später durch echte Läufe ersetzt).

**Done wenn:** Beide CNN-Strategien sind im Browser im Dropdown wählbar, der
Benchmark zeigt sie in der Vergleichstabelle, und für die Conv-Variante
existiert eine Trainings-Anleitung.

---

## Offene Fragen / Risiken
- **Inferenzkosten**: Faltung übers ganze Brett je Tick ist teurer als der
  20-Input-MLP. Bei Preset-Größen (bis 32×24) und kleinen Kanalzahlen (≤8)
  unkritisch erwartet — in Phase C.2 messen (besonders WASM + AlphaZero-CNN,
  wo die Suche viele Forward-Passes macht).
- **Kanalzahl/Tiefe**: klein starten (z. B. 4→8→8), erst bei Bedarf wachsen
  (vgl. Run 029: Kapazität zahlt sich erst mit genug Information aus).
- **Pool+Readout reicht?**: Falls der globale Pool zu grob ist, optional ein
  zweiter Readout-Punkt (Futterzelle) — bewusst erst nach Messung.
