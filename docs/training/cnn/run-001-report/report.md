# Training Report: Conv-Netz — Run 001 (Behavior Cloning)

**Datum**: 2026-06-25
**Ziel**: Erstes echtes Training der einbettbaren Hex-Conv-Strategie. Ansatz:
**Behavior Cloning** des stärksten klassischen Autopiloten (A*-`PathPlanner`
+ Tail-Check). Sekundär getestet: Hochgewichten der futter-annähernden Züge
gegen den dokumentierten „sicheres Kreisen"-Kollaps.

Referenz (Smoke vorher): zufälliges Netz ~0–2 Punkte.

---

## 1. Setup

| Parameter | Wert |
|---|---|
| Experte | A*-`PathPlanner` (+ Tail-Check) via `expert_rollout` |
| Daten | 150 Spiele × 2 Topologien (Walls+Torus), Seeds 0..149, max 2000 Ticks |
| Datensatz | **260 356** Zustände (130k Walls / 130k Torus), **70–71 % futterwärts** |
| Architektur | 4 Kanäle → conv 4→16 → conv 16→16 → Head 32→24→6 (~3 200 Params) |
| Label | absolute Richtung des Experten (Index in `Direction::ALL`) |
| Training | Adam, lr 1e-3, Batch 256, 40 Epochen, 10 % Val-Split |
| Varianten | `tw=1` (reines BC) vs. `tw=4` (futterwärts-Züge 4× gewichtet) |

Kanäle: Körper / Kopf / Futter / Topologie (konstant 1=Walls, 0=Torus).
Größenunabhängig: Kopfzellen-Readout ⊕ Global-Pool → Dense-Head.

## 2. Verlauf

Beide Varianten erreichen ~64 % **Imitations-Genauigkeit** (Top-1 gegen den
Planner) und sättigen dort — `tw=1` best val **0.646**, `tw=4` best val **0.639**.
Die Imitationsgenauigkeit ist gedeckelt, weil der Planner unter gleich sicheren
Zügen quasi-beliebig wählt (Tail-Chasing) — exakt nachahmbar ist das nicht.

## 3. Benchmark (`example benchmark`, 16×12, 10000 Ticks)

| Netz | Walls | Periodic | ⌀ Ticks (W/P) |
|---|---:|---:|---:|
| Smoke (zufällig) | ~0 | ~2 | 2000 / 2000 |
| BC `tw=1` (30 Spiele) | 8.23 | 7.17 | 6749 / 9170 |
| **BC `tw=4` (40 Spiele) — deployed** | **11.72** | **8.55** | **5661 / 8700** |
| Referenz Neural Net (Sensorstrahlen) | 93.2 | 131.1 | — |
| Referenz Pfadplaner (Experte) | 69.0 | 93.9 | — |

## 4. Analyse — BC überlebt, frisst aber nicht

- **Der „sicheres Kreisen"-Kollaps, sichtbar gemacht**: Das Netz überlebt
  extrem lange (5661 / 8700 Ticks, mehr als jede andere Strategie außer
  Hamilton), frisst aber kaum (≈10 Punkte). Es liest Körper/Wand sauber (kein
  vorzeitiger Tod), committet aber nicht aufs Futter.
- **Überraschend trotz futterlastiger Daten**: Die Labels sind zu **70 %**
  futterwärts — der Kollaps kommt also *nicht* aus überwiegend defensiven
  Trainingsdaten. Beim Inferenz-Argmax über die sicheren Züge gewinnt zu oft
  die konservative Heading-Fortsetzung. Das Hochgewichten (`tw=4`) half nur
  moderat (Walls +43 %: 8.2→11.7; Periodic ≈ gleich).
- **Bestätigt die Projekt-Lehre**: Reine Imitation/sichere Ziele kollabieren
  hier zum Kreisen — derselbe Effekt, gegen den AlphaZero ein **dichtes
  Futter-Annäherungs-Reward** + Value/MCTS setzt. Für ein *starkes* Conv-Netz
  ist der nächste Schritt RL/Self-Play (AlphaZero-Conv), nicht mehr BC.
- **Einbettbarkeit verifiziert**: Roundtrip Torch↔Rust < 1.2e-7; das trainierte
  `.cnn` lädt, spielt legal und deterministisch (nativ + WASM).

## 5. Projektstand

| Strategie | Methode | Walls | Periodic | Status |
|---|---|---:|---:|---|
| **Conv-Netz** | BC (A*), `tw=4` | 11.72 | 8.55 | **deployed (Run 001)** |
| AlphaZero-Conv | — (Smoke) | 0.10 | 1.32 | untrainiert, Self-Play folgt |

Das Conv-Netz ist als Strategie funktionsfähig und einbettbar, aber spielerisch
schwach. Ein starker Lauf braucht RL statt BC.

## 6. Dateien

| Datei | Beschreibung |
|---|---|
| `python/training-out/cnn/best.cnn` | BC `tw=1` (Walls 8.23 / Periodic 7.17) |
| `python/training-out/cnn/best_tw.cnn` | **BC `tw=4` — deployed** (Walls 11.72 / Periodic 8.55) |
| `crates/snake-core/assets/cnn/best.cnn` | **Run 001 (`tw=4`) — deployed** |

## 7. Nächste Schritte

- **AlphaZero-Conv per Self-Play** trainieren (das Conv-Netz als Policy/Value
  im MCTS, dichtes Futter-Reward) — der wahrscheinlich tragfähige Weg zu einem
  starken Brett-Vision-Netz. Braucht ein Conv-Self-Play-Binding analog
  `az_selfplay`.
- Optional: ConvNet auf **relative** Aktionen umstellen (wie alle anderen
  Netze) — könnte den Argmax-Bias zur Heading-Fortsetzung mildern.
