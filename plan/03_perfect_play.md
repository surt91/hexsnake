# Plan 03 — Perfektes Spiel aus einem gelernten Modell

Ergänzt [`plan/01_snake.md`](01_snake.md) und [`plan/02_cnn.md`](02_cnn.md).
Ziel: ein **trainiertes Netz**, das HexSnake **perfekt** spielt — Status
`Won`, Brett komplett gefüllt (16×12 ⇒ Score 189) — in **beiden Topologien**,
mit purer Rust/WASM-Inferenz wie bisher.

## Ausgangslage & Gap-Analyse

| Spieler | Walls ⌀ | Periodic ⌀ | Quelle |
|---|---|---|---|
| AlphaZero-light (MLP, Run 029) | 53 | 75.5 | deployed |
| AlphaZero-Conv (Run AZ-001) | 4 (kreist) | 77.5 | deployed |
| Pfadplaner (A*, algorithmisch) | ~145 | ~188 | Benchmark Phase 4 |
| **HamiltonRider (algorithmisch)** | **~180** | **189 = perfekt** | Benchmark Phase 7 |
| Perfekt (16×12) | 189 | 189 | — |

Die gelernten Netze stehen bei ~1/3 des Maximums; die Lücke ist kein
Feintuning-Problem. Der entscheidende Perspektivwechsel:

> **„Perfekt" ist kein Reward-Maximierungs-, sondern ein
> Fehlerraten-Problem.** Ein Perfect Game dauert ~5 000–10 000 Ticks; *ein*
> falscher Zug im Endgame beendet die Partie. Die Policy braucht also eine
> Fehlerrate ≲ 10⁻⁴ pro Zug — Größenordnungen unter dem, was Self-Play-RL
> hier bisher erreicht. Zusätzlich sieht Self-Play die kritischen
> Endgame-Zustände (Schlange > 50 % des Bretts) praktisch nie, weil es sie
> nie erspielen kann (Henne-Ei, vgl. Run AZ-002).

Daraus folgen drei Ansätze mit absteigender Erfolgswahrscheinlichkeit fürs
wörtliche Ziel und aufsteigendem „RL-Ruhm":

1. **Ansatz A — Hamilton-Distillation (DAgger)**: den beweisbar perfekten
   `HamiltonRider` in ein Conv-Netz destillieren. Hauptwette.
2. **Ansatz B — AlphaZero-Conv + Endgame-Curriculum**: Self-Play, aber mit
   Startzuständen aus Lehrer-Partien aller Schlangenlängen. RL-Pfad.
3. **Ansatz C — Gelernte Shortcut-Policy über dem Hamilton-Zyklus**:
   perfekt *by construction* (Sicherheits-Maske), gelernt wird nur die
   Effizienz. Garantierter Fallback.

### Warum BC diesmal funktionieren sollte (A vs. Run 001)

Das gescheiterte BC (Run 001) klonte den **A\*-Pfadplaner**: dessen Labels
sind mehrdeutig (viele gleich gute Züge), global inkonsistent (Pfad hängt
von der ganzen Historie ab) und decken Off-Policy-Zustände nicht ab —
64 % Accuracy, Kreis-Kollaps. Der **HamiltonRider** ist das Gegenteil:

- **deterministisch und global konsistent** — der Zyklus ist eine statische
  Funktion des Bretts; das Ziel-Label ist fast immer „nächste Zelle im
  Zyklus", gelegentlich ein Shortcut. Sehr niedrige Label-Entropie.
- **von jedem Zustand aus definiert** — `HamiltonRider::new(board)` +
  `next_move(state)` funktioniert von beliebigen Positionen. Damit ist
  **DAgger** möglich: der *Student* spielt, der Lehrer labelt jeden
  besuchten Zustand — das behebt den Distribution Shift, die klassische
  BC-Todesursache.
- **inhärent sicher** — wer dem Zyklus folgt, stirbt nie; kleine
  Abweichungen werden vom nächsten DAgger-Zyklus korrigiert.

### Neue Eingaberepräsentation: Vacate-Time-Ebene

Wir sind nicht an die bisherigen Kanäle gebunden. Die entscheidende
Information für sicheres Packen ist nicht „Zelle belegt?", sondern **„wann
wird sie frei?"** — exakt die Größe, mit der `shortcut_is_safe` und das
zeitbewusste A* rechnen. Deshalb ersetzt/ergänzt eine **Vacate-Time-Ebene**
(je Körperzelle: Ticks bis der Schwanz sie räumt, normiert auf die
Schlangenlänge; frei = 0) die binäre Körper-Ebene. Kanäle neu:

- `vacate` — normierte Frei-ab-Zeit (Kopf = 1.0, Schwanzspitze ≈ 1/len),
- `head` — Kopfzelle (1.0),
- `food` — Futterzelle (1.0),
- `topology` — konstante Ebene (1.0 Walls / 0.0 Torus).

Das `.cnn`-Format trägt die Kanalzahl bereits im Header — kein
Formatbruch; die bestehenden 4-Kanal-Assets bleiben ladbar. Das
Hunger-Feature bleibt draußen (Run 026: netto schädlich).

---

## Phase 0 — Messlatte & Infrastruktur

Erst messen können, dann trainieren.

- [ ] **Perfect-Rate im Benchmark**: `examples/benchmark.rs` (und der
      Skill `/benchmark`) melden zusätzlich `won%` (Status `Won`) und
      ⌀-Ticks-bis-Sieg je Strategie/Topologie. Referenzlauf: HamiltonRider
      muss 100 % zeigen (Torus) — das validiert die Metrik.
- [ ] **`bench_cnn`-Example** analog `bench_mlp`: beliebige `.cnn`-Datei
      ohne Rebuild benchmarken (Perfect-Rate inklusive).
- [ ] **Erfolgskriterium festnageln** (in diesem Plan, §Risiken):
      *Primärziel* ≥ 95 % Perfect-Rate über 100 Seeds auf 16×12, beide
      Topologien, mit purem Netz-Argmax über die sicheren Züge
      (Standard-Masking wie bei allen Netz-Strategien, keine Suche).
      *Stretch*: 100 % sowie 24×18.
- [ ] **Vacate-Time-Ebene** in `snake-core` (Plane-Builder) und im
      PyTorch-Spiegel (`hexsnake_rl/hexconv.py`); Kanalzahl-Parameter
      durchziehen, `verify_cnn_roundtrip.py` deckt die neue Ebene ab.
- [ ] **Blog-Notiz**: Gap-Analyse + Fehlerraten-Argument festhalten.

**Done wenn:** Der Benchmark weist Perfect-Rate aus (Hamilton = 100 % auf
dem Torus), und ein 5-Kanal-Netz läuft bitgenau durch den Roundtrip-Test.

## Phase A — Hamilton-Distillation mit DAgger (Hauptwette)

- [ ] **`hamilton_rollout`-Binding** (snake-py, analog `expert_rollout`):
      Lehrer-Partien beider Topologien, Zustände + **absolute**
      Lehrer-Richtung als Label, GIL-frei.
- [ ] **`dagger_rollout`-Binding**: nimmt ein Student-`.cnn`, lässt den
      *Studenten* (Argmax über sichere Züge) spielen und labelt jeden
      besuchten Zustand mit dem Lehrer-Zug. Rückgabe wie oben; komplett in
      Rust, parallel über Partien.
- [ ] **`train_distill.py`**: Runde 0 = reines BC auf Lehrer-Partien;
      danach DAgger-Iterationen (Aggregat-Datensatz wächst). Metriken je
      Runde: Per-Move-Agreement, greedy Perfect-Rate/⌀-Score auf
      Eval-Seeds (Deployment-Modus! — Lehre aus Run 022–025).
      Checkpoint-Auswahl nach Perfect-Rate, Tiebreak ⌀-Ticks.
- [ ] **Fehleranalyse-Werkzeug**: Zustände, in denen der Student vom
      Lehrer abweicht *und* die Partie danach verliert, als Seeds/Ticks
      dumpen — gezielte DAgger-Nachschulung statt blindem Mehr-Budget.
- [ ] **Smoke-Run** (Agent) + `docs/training/distill/guide.md` (Skill
      `/training-docs`); Architektur klein starten (≈ Run-AZ-001-Größe),
      erst bei Accuracy-Plateau wachsen.
- [ ] **Echter Lauf (Nutzer)**, Report, Deployment als `ConvNet`-Asset
      (ersetzt das schwache BC-Netz aus Run 001) — oder als eigener
      Dropdown-Eintrag, falls beide sehenswert sind.
- [ ] **Blog-Notizen**: DAgger-Verlauf (Agreement vs. Perfect-Rate!),
      Vacate-Ebene-Ablation, Vergleich zu Run 001.

**Done wenn:** Das destillierte Netz erreicht das Primärziel aus Phase 0
(≥ 95 % Perfect-Rate, 16×12, beide Topologien) — oder die Fehleranalyse
zeigt ein strukturelles Limit, das dokumentiert ist (→ Gate für B/C).

## Phase B — AlphaZero-Conv mit Endgame-Curriculum (RL-Pfad)

Parallel zu A startbar (Nutzer-Hardware), aber A hat Priorität. Der
Blog-Mehrwert: *lernt* RL Perfektion, statt sie zu imitieren?

- [ ] **Start-State-Sampler**: `az_conv_selfplay --start-from-teacher` —
      Startzustände aus Lehrer-Partien (HamiltonRider), Schlangenlänge
      gleichverteilt (oder endgame-gewichtet) von 3 bis ~90 % des Bretts.
      Bricht das Henne-Ei aus Run AZ-002: Packen wird ab Iteration 1
      trainiert (Idee: „Backplay"/Curriculum aus einer Demonstration,
      Salimans & Chen 2018).
- [ ] **Rezeptivfeld**: Option auf tieferen Conv-Stack
      (`--conv-channels 16 16 16 …`), damit der Kopf Wände früh sieht
      (Hypothese aus Run AZ-001); WASM-Inferenzkosten messen, bevor
      eingebettet wird.
- [ ] **Value-Target fürs Endgame** prüfen: Return-Normierung, die „Brett
      gefüllt" klar von „lange überlebt" trennt (Won-Bonus im Reward der
      Suche), sonst bleibt Kreisen lokal ununterscheidbar.
- [ ] **Echter Lauf (Nutzer)** mit `--max-hours`-Budget; Abbruchkriterium
      vorab festlegen (z. B. Walls-⌀ < 60 nach 6 h ⇒ Ansatz einfrieren,
      Ergebnis dokumentieren — Lehre aus Run 026/027: Compute ersetzt
      keinen Lernhebel).
- [ ] **Vergleichstabelle A vs. B** (Perfect-Rate, ⌀-Score, ⌀-Ticks,
      Params, Trainingskosten) im Report + Blog-Notizen.

**Done wenn:** Der Walls-Kreis-Kollaps ist gebrochen (Walls-⌀ deutlich
über Run 029) und die Perfect-Rate ist messbar > 0 — *oder* das negative
Ergebnis ist mit Abbruchkriterium sauber dokumentiert.

## Phase C — Gelernte Shortcut-Policy (garantierter Fallback)

Nur falls A **und** B das Primärziel verfehlen — oder als Bonus-Strategie.
Ehrliche Einordnung: Perfektion kommt hier aus der Maske, gelernt wird die
Geschwindigkeit.

- [ ] **Aktionsraum umbauen**: Kandidaten = alle per `shortcut_is_safe`
      beweisbar sicheren Sprünge entlang der Zyklusordnung (+ „Zyklus
      folgen"). *Jede* Policy über diesem Aktionsraum spielt perfekt.
- [ ] **Kleines Scoring-Netz** (MLP über Kandidaten-Features:
      Zyklus-Distanzgewinn, Vacate-Slack, Futter-Nähe, Schlangenlänge)
      wählt den Kandidaten; Training per ES in `snake-train`
      (vorhandener `evolve`-Kern), Fitness = −Ticks bis `Won`.
- [ ] **Benchmark**: 100 % Perfect-Rate (per Konstruktion) und weniger
      ⌀-Ticks als der heuristische HamiltonRider; sonst lohnt der Eintrag
      nicht.
- [ ] **Blog-Notiz**: „perfekt by construction, gelernt effizient" als
      Kontrast zu A/B.

**Done wenn:** Eine gelernte Strategie gewinnt jede Partie und ist
messbar schneller als der HamiltonRider — oder Phase C wurde bewusst
nicht gebraucht (A/B erfolgreich) und ist als offen markiert.

---

## Offene Fragen / Risiken

- **Zählt das Safety-Masking?** Alle Netz-Strategien im Repo maskieren
  sofort tödliche Züge — das bleibt der Standard und gilt weiterhin als
  „das Netz spielt". Tiefere Suche (MCTS) zur Inferenzzeit wäre dagegen
  eine andere Kategorie und wird für das Primärziel *nicht* verwendet.
- **Fehlerraten-Mathematik**: 95 % Perfect-Rate über ~7 000 Züge heißt
  Per-Zug-Fehler ≲ 7·10⁻⁶ auf dem Spielpfad — erreichbar, weil die
  Lehrer-Policy fast deterministisch ist und Abweichungen selten fatal
  sind (DAgger korrigiert zurück auf den Zyklus). Wenn nicht: Gate zu C.
- **Generalisierung über Brettgrößen**: Das Conv-Netz ist größenagnostisch,
  der Zyklus aber brettspezifisch. Primärziel ist 16×12; ob *ein* Netz
  auch 24×18 perfekt spielt (Stretch), entscheidet sich erst nach A.
  Notfalls: Training auf gemischten Größen.
- **WASM-Budget**: Ein Perfect Game im Browser heißt ~10⁴ Conv-Forwards;
  bei den kleinen Netzen (~3–10 k Params) unkritisch erwartet, aber vor
  dem Einbetten auf Preset-Größen messen (bes. bei tieferem Stack aus B).
- **Ungerade Zeilenzahl**: Der Serpentinen-Zyklus braucht gerade
  Zeilenzahl (alle Presets kompatibel). Das destillierte Netz erbt diese
  Einschränkung implizit über die Trainingsdaten — für freie Feldgrößen
  bleibt es Best-Effort, kein Perfektionsversprechen.
- **Lehrer-Deckelung**: Der HamiltonRider ist auf Walls im Benchmark bei
  ~180 (Tick-Limit, keine Tode). Für die Lehrer-Rollouts das Tick-Limit
  hoch genug setzen (z. B. 20 000), damit nur `Won`-Partien als
  Trainingsmaterial dienen.
