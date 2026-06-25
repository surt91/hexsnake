# Training Report: AlphaZero-Conv — Run AZ-001 (Self-Play)

**Datum**: 2026-06-25
**Ziel**: Das erste echte **RL/Self-Play**-Training des Brett-Vision-Netzes.
Das Conv-Netz (ganzes Brett als Input) wird als Policy/Value-Evaluator in
*genau dieselbe* MCTS wie `AlphaZeroLite` gesteckt (`AlphaZeroConv`) und per
Self-Play trainiert — der von Run 001 (Behavior Cloning) empfohlene nächste
Schritt: „ein *starkes* Conv-Netz braucht RL, nicht BC".

Referenz (vorher eingebettet): deterministisches Smoke-Artefakt, Walls 0.1 /
Periodic 1.3. Behavior-Cloning-Conv-Netz (Run 001, anderer Slot): Walls 11.2 /
Periodic 7.2.

---

## 1. Pipeline (neu gebaut)

Die AlphaZero-Conv-**Trainings**-Pipeline existierte noch nicht (nur die
Inferenz-Strategie + Smoke-Netz). Neu in dieser Session:

- `snake-core`: `self_play_conv_with_rewards` — dieselbe MCTS + dichte
  Reward-Formung wie das MLP-Self-Play, aber das Conv-Netz als Evaluator; jeder
  Sample trägt das **ganze Brett-Grid** plus die **absolute** Besuchsverteilung
  (relative MCTS-Counts in den absoluten Frame zurückrotiert) und den
  tanh-Return.
- `snake-py`: Binding `az_conv_selfplay` (Netz als `.cnn`-Text, GIL frei).
- `python/train_alphazero_conv.py`: Self-Play in Rust, Gradientenschritt in
  Torch — Policy-CE gegen MCTS-Besuche (mit per-Sample maskiertem Reverse-Zug),
  Value-MSE gegen den Return; Export als `.cnn`; greedy-Eval-Checkpointing wie
  beim MLP-Trainer.
- Roundtrip Torch↔Rust (`cnn_forward`): **1.19e-7**; Export-Self-Check je Lauf.

## 2. Setup

| Parameter | Run AZ-001 | Run AZ-002 (Vergleich) |
|---|---|---|
| Architektur | 4-Kanal → conv 4→16 → 16→16 → Head 32→24→**7** (~3 200 P) | conv 24/24, Head 48→32→7 (~6 550 P) |
| `--sims` (= `embedded()`) | 24 | 24 |
| `--games-per-iter` / `--max-ticks` | 96 / 1500 | 96 / 1500 |
| Boundary | mixed (Walls+Torus) | mixed |
| Buffer / Epochen / LR | 80 000 / 4 / 1e-3 | 80 000 / 4 / 1e-3 |
| Topology-Balancing | aus | **an** (Minderheit oversamplen) |
| Eval | greedy, Seeds 0..11, 3000 Ticks, alle 5 Iter | dito |
| Hardware | 32 Cores (Self-Play CPU), ~50 s/Iter | langsamer (~4 min/Iter) |

Kanäle: Körper / Kopf / Futter / Topologie (konstant 1=Walls, 0=Torus).
Größenunabhängig via Kopfzellen-Readout ⊕ Global-Pool → Dense-Head.

## 3. Verlauf

**Run AZ-001** lernt schnell und sauber: Self-Play-Score 1 → 35 in ~20 Iter,
Policy-Loss 1.57 → 0.01 (konvergiert), Value-Loss ~0. Bester Eval-Checkpoint
**iter 40: W 4.3 / P 77.5 / avg 40.9** — Periodic erreicht das Niveau des
MLP-AlphaZero. Walls bleibt über den ganzen Lauf bei **2–6**.

**Run AZ-002** (größeres Netz + Balancing als Walls-Fix-Versuch) konvergiert
*schlechter*: bester Checkpoint iter 25 **W 1.5 / P 61.4 / avg 31.5**. Das
Balancing hat Walls **nicht** gehoben (eher verschlechtert), das größere Netz
brachte bei weniger Iterationen (langsamer) ein niedrigeres Periodic. **Run
AZ-001 wird deployed.**

## 4. Benchmark (`example benchmark`, 15 Partien, 3000 Ticks, 16×12)

| Strategie | Walls | Periodic | ⌀ Ticks (W/P) |
|---|---:|---:|---:|
| Smoke (vorher eingebettet) | 0.10 | 1.32 | — |
| Conv-Netz (BC, Run 001) | 11.20 | 7.20 | 1954 / 2775 |
| **AlphaZero-Conv (AZ-001) — deployed** | **4.13** | **77.47** | **3000 / 2036** |
| AlphaZero-light (MLP, Referenz) | 62.40 | 77.00 | 1294 / 1751 |
| Pfadplaner (Experte) | 67.47 | 92.47 | 883 / 846 |
| Neural Net (Sensorstrahlen) | 87.80 | 131.93 | — |

## 5. Analyse — Periodic gelöst, Walls bleibt „sicheres Kreisen"

- **Periodic: vollwertig.** Mit 77.47 spielt das Conv-Netz Periodic **gleichauf
  mit dem MLP-AlphaZero** (77.00) und 10× stärker als das BC-Conv-Netz (7.2).
  Das dichte Futter-Annäherungs-Reward in der Suche löst den Kreis-Kollaps, der
  reines BC (Run 001) deckelte — auf dem Torus, wo Futter-Verfolgung nie an eine
  Wand führen kann.
- **Walls: der Kreis-Kollaps überlebt — wörtlich.** Auf Walls erreicht das Netz
  die volle **3000-Tick-Grenze** (⌀ Ticks 3000.0) bei nur **4 Punkten**. Es
  stirbt also *nicht* in Ecken — es **kreist sicher** und committet nie aufs
  Futter. Dieselbe „sicheres Überleben statt Fressen"-Pathologie wie beim BC,
  aber **topologie-selektiv**: Auf Walls riskiert jeder Futter-Zug nahe der Wand
  den Tod, also bleibt die gelernte Policy defensiv; der Todes-Malus überwiegt
  das Annäherungs-Reward. Auf dem Torus gibt es dieses Risiko nicht.
- **Balancing widerlegt als Walls-Fix.** Hypothese war Buffer-Schieflage (Torus
  spielt länger → mehr Samples). Das per-Epoche-Oversampling der Walls-Samples
  (Run AZ-002) hob Walls nicht — die wenigen, todeslastigen Walls-Trajektorien
  zu wiederholen erzeugt kein Signal, das nicht da ist. Es ist ein **Self-Play-
  Bootstrap-Problem**, keine reine Datenbalance: Das Netz kann Walls nicht gut
  spielen → erzeugt keine guten Walls-Daten → lernt Walls nicht.
- **MLP vs. Conv auf Walls.** Der MLP-AlphaZero schafft Walls 62, das Conv-Netz
  4. Der Sensor-MLP bekommt Wand-Abstände *direkt* als Strahlen-Features; das
  Conv-Netz muss Wände aus dem Grid + 2-Hop-Faltung (kleiner Rezeptivbereich)
  selbst ableiten. Plus: Periodic-Self-Play dominiert das Lernen, weil es
  sofort funktioniert.
- **Einbettbarkeit verifiziert.** Das `.cnn` lädt, spielt legal und
  deterministisch (`embedded_conv_plays_legally_and_deterministically`), nativ +
  WASM, sims 24 == `AlphaZeroConv::embedded()`.

## 6. Projektstand

| Conv-Strategie | Methode | Walls | Periodic | Status |
|---|---|---:|---:|---|
| Conv-Netz | BC (A*), `tw=4` | 11.20 | 7.20 | deployed (Run 001) |
| **AlphaZero-Conv** | **Self-Play (AZ-001)** | **4.13** | **77.47** | **deployed (AZ-001)** |

AlphaZero-Conv ist damit das **mit Abstand stärkste Brett-Vision-Netz auf
Periodic** (gleichauf MLP) und ersetzt das Smoke-Netz im Slot. Walls bleibt die
offene Baustelle — strukturell dieselbe wie beim BC, nur topologie-selektiv.

## 7. Dateien

| Datei | Beschreibung |
|---|---|
| `python/training-out/az-conv/run-001/best.cnn` | **iter 40 (W 4.13 / P 77.47) — deployed** |
| `python/training-out/az-conv/run-001/run.log` | Trainings-Log AZ-001 |
| `python/training-out/az-conv/run-002/best.cnn` | iter 25 (avg 31.5), Balancing-Vergleich |
| `crates/snake-core/assets/alphazero-cnn/best.cnn` | **AZ-001 best — deployed** |

## 8. Nächste Schritte (Walls)

- **Größerer Rezeptivbereich**: mehr Conv-Layer (3–4 Hops), damit der Kopf
  Wände früh genug „sieht", um Sackgassen zu antizipieren — der wahrscheinlichste
  architektonische Hebel.
- **Curriculum / Walls-Übergewicht im Self-Play**: erst Walls-only anlernen (das
  Bootstrap-Henne-Ei brechen), dann mixed feinschleifen; oder mehr Walls- als
  Torus-Partien je Iteration statt nur Sample-Balancing.
- **Stärkerer Todes-Malus-Ausgleich**: das Annäherungs-Reward nahe Wänden
  anheben oder den Todes-Malus dämpfen, damit die Walls-Policy aus dem
  defensiven Kreisen herausfindet.
- **Mehr Iterationen**: Conv-Self-Play ist pro Iter ~30× teurer als der MLP
  (ganzes Brett statt Strahlen); der MLP-AlphaZero brauchte ~1600 Iter für
  starkes Walls. In 4,5 h erreicht Conv nur ~50 — evtl. schlicht zu wenig Budget
  für die harte Topologie.
