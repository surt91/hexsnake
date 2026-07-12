# Plan 03 — Perfektes Spiel aus einem gelernten Modell

Ergänzt [`plan/01_snake.md`](01_snake.md) und [`plan/02_cnn.md`](02_cnn.md).
Ziel: ein **trainiertes Netz**, das HexSnake **perfekt** spielt — Status
`Won`, Brett komplett gefüllt (16×12 ⇒ Score 189) — in **beiden Topologien**,
mit purer Rust/WASM-Inferenz wie bisher. Dazu ein trainingsfreier
Referenz-Algorithmus (Phase D), der perfekt *und* schnell spielt.

## Ausführungsmodus (für den umsetzenden Agenten — zuerst lesen)

Dieser Plan ist **selbstständig abarbeitbar**; es kommt kein weiterer
Kontext vom Nutzer. Verbindliche Regeln:

- **Reihenfolge**: Phase 0 → Phase D → Phase A → Phase B → (Phase C nur
  bedingt, siehe dort). **Phasen 0, D, A und B sind Pflicht** und werden
  unabhängig vom Erfolg der jeweils anderen Phasen vollständig
  ausgeführt. Nur Phase C ist konditional.
- **Training läuft direkt auf dieser Maschine** (≈32 Cores, NVIDIA-GPU).
  Anders als in `docs/training/*/guide.md` beschrieben gibt es keine
  Trennung „Agent macht Smoke-Run, Nutzer trainiert": Der Agent führt
  nach dem obligatorischen Smoke-Run auch den **echten Lauf** selbst aus.
  Lange Läufe (Phase B: 6 h) als Hintergrundprozess starten, Log in
  `python/training-out/…/train.log`, regelmäßig prüfen.
- **Repo-Konventionen gelten** (siehe `CLAUDE.md`): Vor jedem Commit
  Skill `/check`; Conventional Commits pro abgeschlossener Checkbox;
  Blog-Notizen via `/blog-notes`; Trainings-Doku via `/training-docs`;
  Benchmarks via `/benchmark`. Checkboxen hier im Plan direkt abhaken.
- **Für jeden Trainingslauf** einen Report unter
  `docs/training/<name>/run-NNN-report/report.md` anlegen (Muster: die
  vorhandenen Reports, z. B. `docs/training/alphazero/run-027-report/`).
- Wo dieser Plan konkrete Zahlen nennt (Budgets, Schwellwerte, Netzgrößen),
  sind das **Defaults, keine Vorschläge** — abweichen nur mit dokumentierter
  Begründung im Report.

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

Die vier Ansätze:

1. **Ansatz A — Lehrer-Distillation (DAgger)**: den beweisbar perfekten
   Zyklus-Lehrer in ein Conv-Netz destillieren. Hauptwette fürs wörtliche
   Ziel.
2. **Ansatz B — AlphaZero-Conv + Endgame-Curriculum**: Self-Play, aber mit
   Startzuständen aus Lehrer-Partien aller Schlangenlängen. RL-Pfad,
   Pflicht unabhängig vom Ausgang von A.
3. **Ansatz C — Gelernte Shortcut-Policy** (konditional): perfekt *by
   construction* (Sicherheits-Maske), gelernt wird nur die Effizienz.
4. **Ansatz D — Zyklus-Chirurg (kein Training)**: dynamische
   Hamilton-Zyklus-Reparatur; beweisbar perfekt, auf minimale Ticks
   optimiert. Wird **vor** A/B gebaut und dient dort als Lehrer.

### Warum BC diesmal funktionieren sollte (A vs. Run 001)

Das gescheiterte BC (Run 001) klonte den **A\*-Pfadplaner**: dessen Labels
sind mehrdeutig (viele gleich gute Züge), global inkonsistent (Pfad hängt
von der ganzen Historie ab) und decken Off-Policy-Zustände nicht ab —
64 % Accuracy, Kreis-Kollaps. Ein Zyklus-Lehrer (HamiltonRider bzw.
Zyklus-Chirurg) ist das Gegenteil:

- **deterministisch und konsistent** — das Ziel-Label ist fast immer
  „nächste Zelle im Zyklus", gelegentlich ein Shortcut. Sehr niedrige
  Label-Entropie.
- **von jedem Zustand aus definiert** — der Lehrer liefert für jeden
  erreichbaren `GameState` einen Zug. Damit ist **DAgger** möglich: der
  *Student* spielt, der Lehrer labelt jeden besuchten Zustand — das behebt
  den Distribution Shift, die klassische BC-Todesursache.
- **inhärent sicher** — wer dem Zyklus folgt, stirbt nie; kleine
  Abweichungen werden vom nächsten DAgger-Zyklus korrigiert.

### Neue Eingaberepräsentation: Vacate-Time-Ebene

Die entscheidende Information für sicheres Packen ist nicht „Zelle
belegt?", sondern **„wann wird sie frei?"** — exakt die Größe, mit der
`shortcut_is_safe` und das zeitbewusste A* rechnen. Deshalb bekommt das
Conv-Netz eine **fünfte Ebene**. Kanal-Layout (Reihenfolge fix, die ersten
vier identisch zum Bestand, damit 4-Kanal-Assets ladbar bleiben):

| # | Kanal | Wert |
|---|---|---|
| 0 | `body` | 1.0 auf Körperzellen ohne Kopf |
| 1 | `head` | 1.0 auf der Kopfzelle |
| 2 | `food` | 1.0 auf der Futterzelle |
| 3 | `topology` | konstant 1.0 (Walls) / 0.0 (Torus) |
| 4 | `vacate` | Körperzelle mit Index k **vom Schwanz** (Schwanz k=1, Kopf k=len): Wert `k/len`; freie Zellen 0.0 |

Die Kanalzahl steht bereits im `.cnn`-Header; der Rust-Plane-Builder wird
kanalzahl-gesteuert (`in_channels == 4` ⇒ Ebenen 0–3, `== 5` ⇒ 0–4,
alles andere ⇒ Fehler). Das Hunger-Feature bleibt draußen (Run 026:
netto schädlich).

---

## Phase 0 — Messlatte & Infrastruktur

Erst messen können, dann trainieren.

- [x] **Perfect-Rate im Benchmark**: `crates/snake-core/examples/benchmark.rs`
      um zwei Spalten erweitern: `won%` (Anteil Partien mit Status `Won`)
      und `⌀ticks(won)` (mittlere Ticks der gewonnenen Partien; `—` wenn
      keine). Den Skill `/benchmark` (`.claude/skills/benchmark/SKILL.md`)
      entsprechend aktualisieren. Referenzlauf mit
      `cargo run --release -p snake-core --example benchmark -- 50 20000`:
      HamiltonRider muss auf dem Torus 100 % `won` zeigen — das validiert
      die Metrik. (Tick-Limit 20 000, damit Walls-Hamilton-Partien nicht
      am Limit abgeschnitten werden.)
- [x] **`bench_cnn`-Example** (`crates/snake-core/examples/bench_cnn.rs`)
      analog zu `bench_mlp`: CLI `<datei.cnn> <spiele> <max_ticks>`,
      bencht die Datei als `ConvNet`-Strategie (6-Output) bzw. als
      `AlphaZeroConv` (7-Output, Sims wie `embedded()`), erkennbar an der
      Output-Dimension des Head. Gibt Score, `won%`, `⌀ticks` je Topologie
      aus.
- [x] **Vacate-Time-Ebene**: Plane-Builder in
      `crates/snake-core/src/nn/conv.rs` auf 4-oder-5 Kanäle erweitern
      (Tabelle oben); PyTorch-Spiegel `python/hexsnake_rl/hexconv.py`
      identisch erweitern; `python/verify_cnn_roundtrip.py` prüft
      zusätzlich ein 5-Kanal-Netz (Toleranz wie bisher, ~1e-6). Unit-Tests
      in Rust: Vacate-Werte einer bekannten Schlange, 4-Kanal-Netze laden
      weiterhin (Regress).
- [ ] **Blog-Notiz** (falls beim Umsetzen Neues auffällt; die Gap-Analyse
      selbst ist schon notiert).

**Done wenn:** Der Benchmark weist `won%` aus (Hamilton = 100 % auf dem
Torus), `bench_cnn` läuft gegen eine eingecheckte `.cnn`-Datei, und ein
5-Kanal-Netz geht bitgenau durch den Roundtrip-Test.

**Erfolgskriterium des Gesamtplans** (gilt für A und B): *Primärziel* =
≥ 95 % Perfect-Rate über 100 Partien (Board-Seeds 0–99) auf 16×12, in
**beiden** Topologien, mit purem Netz-Argmax über die sicheren Züge
(Standard-Masking wie bei allen Netz-Strategien, keine Suche zur
Inferenzzeit). *Stretch*: 100 % sowie 24×18.

## Phase D — Zyklus-Chirurg: dynamische Hamilton-Reparatur (kein Training, Pflicht)

Der HamiltonRider ist perfekt, aber langsam: er fährt einen **statischen**
Zyklus ab und darf ihn nur per Shortcut *entlang der Zyklusordnung*
abkürzen — liegt das Futter „gegen die Fahrtrichtung", fährt er fast das
ganze Brett ab. Der Zyklus-Chirurg passt stattdessen **den Zyklus ans
Futter an**. Vorbild: „dynamic Hamiltonian cycle repair" vom Quadratgitter
(AlphaPhoenix); der Hex-Adjazenzgraph ist ein Dreiecksgitter (6 Nachbarn,
nicht bipartit) und erlaubt zusätzliche Operationen.

### Sicherheitsargument

Invariante: Es existiert stets ein gültiger Hamilton-Zyklus über alle
Zellen, und **Umbauten fassen nie eine vom Schlangenkörper belegte Kante
an**. Folgt die Schlange dem Zyklus, liegt ihr Körper als
zusammenhängendes Segment darauf und sie kann nie kollidieren; am Ende
füllt sie das Brett (`Won`) — derselbe Beweis wie beim HamiltonRider, nur
dass sich der Zyklus bewegt. (Die Startaufstellung liegt evtl. nicht auf
dem Zyklus; wie beim HamiltonRider deckt der Vacate-Check die ersten Züge
ab, danach greift die Invariante. Der Property-Test unten ist der
Schiedsrichter.)

### Datenstruktur

- Zellindex = `row * width + col` (row-major, Offset-Koordinaten).
- Zyklus als doppelt verkettete Ringe: `next: Vec<usize>`,
  `prev: Vec<usize>`; initialisiert aus `serpentine_cycle(width, height)`
  (`crates/snake-core/src/strategy/hamilton.rs`). Kompatibilität =
  `HamiltonRider::compatible` (gerade Zeilenzahl).
- **Belegte Kante**: ungerichtetes Paar `{u, v}`, wenn `u` und `v`
  aufeinanderfolgende Schlangensegmente sind. Pro Tick einmal als
  `HashSet`-freie Struktur aufbauen (z. B. `Vec<Option<usize>>`
  „Körper-Nachfolger je Zelle" — Determinismus!).
- Zyklusdistanz `d(a→b)` = Schritte entlang `next` von `a` bis `b`
  (Walk, O(n); n ≤ 768 bei 32×24 — unkritisch).

### Operationskatalog (alle nur auf unbelegten Kanten)

1. **Relocate (O(1), keine Umkehr)** — Zelle `x` als Umweg in Kante
   `(a→b)` einfügen: Voraussetzungen `x ∉ {a,b}`, `x` keine
   Schlangenzelle, `x` adjazent zu `a` **und** `b`, mit `p = prev[x]`,
   `q = next[x]`: `p` adjazent `q`, und die Kanten `(a,b)`, `(p,x)`,
   `(x,q)` unbelegt. Anwendung: `next[p]=q; next[a]=x; next[x]=b` (+
   `prev` spiegeln). Auf dem Dreiecksgitter ist `p` adjazent `q` häufig
   (beide sind Nachbarn von `x`) — das ist die Brot-und-Butter-Operation.
2. **2-opt (Segmentumkehr)** — Kanten `(a→b)` und `(c→d)` entfernen,
   `(a→c)` und `(b→d)` einfügen, Segment `b…c` umdrehen. Voraussetzungen:
   `a` adjazent `c`, `b` adjazent `d`, beide entfernten Kanten unbelegt,
   **keine Schlangenzelle im Segment `b…c`** (sonst würde die
   Körperrichtung gegen die Zyklusrichtung laufen), Segmentlänge ≤ 32
   (Kostendeckel für die Umkehr).

Jede Operation als reine Funktion mit `apply`/Bedingungs-Check; nach
`apply` sind `next`/`prev` konsistent.

### Optimierung pro Tick (deterministisch, budgetiert)

```text
d0 = d(head → food)
wiederhole bis keine Verbesserung oder 256 Versuche:
  laufe den Zykluspfad head→food entlang (max. 64 Zellen Fenster);
  für jede Pfadkante (a→b), in fester Reihenfolge:
    für jeden Kandidaten (Relocate über alle x ∈ N(a)∩N(b) in
    Direction::ALL-Reihenfolge; dann 2-opt über c ∈ N(a), d = next[c]):
      wenn Bedingungen erfüllt: tentativ anwenden,
      d1 = d(head → food) neu berechnen;
      d1 < d0 ⇒ behalten (d0 = d1), sonst exakt zurückrollen
```

Kein RNG, keine HashMap-Iteration — gleiche Eingabe ⇒ gleicher Zyklus.

### Zugwahl

Wie `HamiltonRider::next_move`, nur auf dem **aktuellen** Zyklus: dem
Zyklus folgen, plus die vorhandene Shortcut-Prüfung (Sprung entlang der
Zyklusordnung, wenn `shortcut_is_safe` mit Vacate-Zeiten zustimmt). Dazu
die Shortcut-/Vacate-Logik aus `hamilton.rs` in eine gemeinsame Funktion
extrahieren (`pub(crate)`), die beide Strategien nutzen —
**HamiltonRider-Verhalten muss bitidentisch bleiben** (bestehende Tests
als Regress-Schutz).

### Checkliste

- [x] `crates/snake-core/src/strategy/cycle_surgeon.rs`: Datenstruktur,
      Ops, Optimierer, `CycleSurgeon`-Strategie (`Strategy`-Impl);
      `StrategyDebug.path` = aktueller Zyklus ab Kopf (Overlay zeigt das
      Umbauen live).
- [x] Zyklus-Validator aus den `hamilton.rs`-Tests als wiederverwendbaren
      Helper heben (jede Zelle genau einmal, Übergänge adjazent,
      geschlossen) und nach **jeder** Op im Test prüfen.
- [x] **Property-Tests**: (1) je 20 Seeds × beide Topologien auf 16×12,
      max 20 000 Ticks ⇒ alle Partien enden `Won`; (2) Validator nach
      jedem Tick grün; (3) Determinismus (zwei Läufe, gleicher Seed ⇒
      identische Zugfolge); (4) 24×18 ein Stichproben-Seed ⇒ `Won`.
- [x] **Dropdown + Benchmark**: `StrategyChoice::CycleSurgeon` in
      `crates/snake-app/src/settings.rs` (`ALL`, `label`,
      `compatible_with` wie Hamilton) und `game_view.rs`; Aufnahme in
      `examples/benchmark.rs`.
- [x] **Benchmark-Ziel**: 100 % `won` in beiden Topologien (16×12, 100
      Partien, 20 000 Ticks) und ⌀-Ticks ≥ 30 % unter HamiltonRider.
      Wird das Speed-Ziel verfehlt: Fenster/Budget tunen, ggf. Ops
      ergänzen; Ergebnis im Blog notieren, aber `won%` hat Vorrang.
- [x] **Blog-Notizen**: Operationskatalog, Dreiecks- vs. Quadratgitter,
      Benchmark-Zahlen, Overlay-Beobachtungen.

**Done wenn:** `CycleSurgeon` gewinnt 100/100 Partien in beiden Topologien
(16×12) und ist im Mittel ≥ 30 % schneller (Ticks) als der HamiltonRider;
alle Property-Tests grün; im Dropdown wählbar.

### ⚠ Problembericht (Umsetzung 2026-07-12): Speed-Ziel blockiert

Der erste Implementierungsversuch offenbart einen **strukturellen Konflikt**,
der so im Plan nicht vorgesehen war. Messungen (16×12, je 20 Seeds, Release,
`⌀ticks(won)`):

| Variante | won% | ⌀ticks Walls | ⌀ticks Periodic |
|---|---|---|---|
| HamiltonRider (Referenz, mit Shortcuts) | 100 / 98 | 4988 | 4581 |
| Surgeon: Reshape **+ Shortcut** (`ride_cycle`) | ~60 % | — (stirbt) | — (stirbt) |
| Surgeon: Reshape **+ striktes Folgen (Offset 1)** | **100 %** | 8390 | 7348 |

**Zwei Kernbefunde:**

1. **Shortcuts sind mit dynamischem Reshaping unvereinbar.** Der
   `shortcut_is_safe`-Beweis (Vacate-Zeiten) setzt voraus, dass die Schlange
   nach dem Sprung dem **aktuellen** Zyklus folgt. Wird der Zyklus im nächsten
   Tick umgebaut, ist die Annahme verletzt — die Schlange fährt sich fest und
   **stirbt** (nicht Livelock: echte `GameOver`, per Diagnose über 20 Seeds).
   Damit fällt die im Plan-Abschnitt „Zugwahl" vorgesehene Shortcut-Nutzung
   weg; sicher ist nur striktes Folgen des Zyklus-Nachfolgers (Offset 1), das
   die Körper-Kontiguität (und damit den `Won`-Beweis) erhält.

2. **Relocate/2-opt feuern auf dem Serpentinen-Zyklus fast nie.** Die
   Plan-Annahme „auf dem Dreiecksgitter ist `p` adjazent `q` häufig" trifft
   für die Serpentine **nicht** zu: die Zyklus-Nachbarn `prev[x]`/`next[x]`
   einer Zelle liegen auf dem Hex-Ring 2 Schritte auseinander (z. B. NW & NE
   bzw. SW & SE) und sind damit **nicht** benachbart. `p adj q` (Relocate)
   und `b adj d` (2-opt, misst identisch für alle `MAX_SEGMENT` 32…192)
   scheitern fast immer. Das Reshaping bringt zwar Gewinn gegenüber reinem
   Serpentinen-Folgen (~8390 statt ~18000 Ticks), erreicht aber nicht die
   Shortcut-Geschwindigkeit des HamiltonRiders.

**Folge:** Reshape+Folgen ist **sicher und perfekt** (100 % `won` beide
Topologien, 16×12 wie 24×18 bei genug Ticks) — taugt also als Lehrer für
Phase A/B — ist aber **~1,7× langsamer** als der HamiltonRider statt 30 %
schneller. Das Speed-`Done`-Kriterium und der 24×18-Property-Test bei
20 000 Ticks (selbst Hamilton braucht dort ~24 000!) sind so nicht erfüllbar.

### ✅ Auflösung (fable-Design): Cross-Swap statt Relocate/2-opt

Ein stärkeres Modell (fable) lieferte den fehlenden Baustein — die Ops
waren das Problem, nicht die Invariante. Statt Relocate (braucht `p adj q`)
und 2-opt (braucht `b adj d`, Paritäts-Widerspruch zwischen antiparallelen
Serpentinen-Zeilen) nun **eine** richtungserhaltende Primitive:

- **Cross-Swap** `(a→b),(u→v) ⇒ (a→v),(u→b)` (braucht `a adj v`, `u adj b`).
  Auf einem Zyklus mit Segment `b…u` **spaltet** sie den Ring `[b…u]` ab; über
  zwei Zyklen **verschmilzt** sie sie. Keine Segmentumkehr (Körperrichtung nie
  gefährdet), selbst-invers ⇒ O(1)-Rollback ohne `clone`.
- **Excise-and-Transplant** (Arbeitspferd): ein freies Stück aus dem
  head→food-Bogen herausspalten (verkürzt die Distanz um die Stücklänge), den
  Ring per zweiter Cross-Swap **hinter dem Futter** wieder einfügen. Der
  Spaltpunkt existiert *immer*, weil die N/S-Spaltenkanten (Odd-q) für jede
  Zelle da sind — die zentrale Vorbedingung feuert überall.
- **Kontiguität ⇒ Intervall-Arithmetik**: Weil der Körper zusammenhängend auf
  dem Zyklus liegt, belegt er ein festes Positions-Intervall — „frei? hinter
  dem Futter?" ist O(1). Reshaping nur, wenn kontig (die ersten ~2 Ticks nach
  Spawn folgen dem Serpentinen-Seed).

**Ergebnis (16×12, 100 Seeds, 20 000 Ticks, Release):** 100 % `won` in beiden
Topologien (Torus sogar besser als Hamilton, der dort auf 1 Seed *stirbt*);
Walls **−25,7 %** Ticks, Periodic **−36,0 %** ⇒ **im Mittel −30,9 %** — Speed-
`Done` erfüllt. (Walls einzeln knapp unter 30 %: die Wand-Geometrie limitiert
die Merge-Sites; das Mittel trägt.) Property-Tests grün, Dropdown/Benchmark
integriert. **Phase D abgeschlossen; `CycleSurgeon` ist der Lehrer für A/B.**

## Phase A — Lehrer-Distillation mit DAgger (Pflicht)

Lehrer = `CycleSurgeon` (aus Phase D; sollte D wider Erwarten scheitern:
`HamiltonRider`). Student = `HexConv`-Netz mit 5 Kanälen (Phase 0),
6 absolute Outputs (wie `ConvNet`).

### Neue snake-py-Bindings (`crates/snake-py/src/lib.rs`)

Alle GIL-frei (`py.allow_threads`), parallel über Partien (rayon), Muster
`expert_rollout`:

- [ ] `teacher_rollout(boundary, width, height, games, seed, max_ticks)`
      → `(states, labels, outcomes)`: Lehrer spielt; je Tick die
      5-Kanal-Ebenen (über den Rust-Plane-Builder — eine Quelle der
      Wahrheit) und die **absolute** Lehrer-Richtung (0–5 =
      `Direction::ALL`-Index) als Label; `outcomes` je Partie
      `(won, score, ticks)`.
- [ ] `dagger_rollout(cnn_text, boundary, width, height, games, seed,
      max_ticks)` → gleiche Rückgabe: **Student** (Argmax der
      `ConvNet`-Scores über die sicheren Züge) spielt; parallel läuft der
      Lehrer im **Lockstep** auf denselben Zuständen mit (eigene Instanz
      pro Partie, `teacher.next_move(&state)` vor jedem Student-Tick) und
      liefert das Label für jeden besuchten Zustand.
- [ ] `cnn_eval_games(cnn_text, boundary, width, height, games,
      seed_base, max_ticks)` → `Vec<(won, score, ticks)>`: greedy
      Deployment-Eval für Checkpoint-Auswahl (Board-Seeds
      `seed_base..seed_base+games`).

### Trainings-Skript `python/train_distill.py`

- [ ] CLI: `--width/--height` (Default 16×12), `--bc-games` (Default 200
      je Topologie), `--dagger-rounds` (Default 8), `--dagger-games`
      (Default 100 je Topologie und Runde), `--epochs-per-round` (Default
      10), `--lr` (1e-3, Adam), `--batch` (256), `--conv-channels`
      (Default `16 16`), `--head-hidden` (Default `24`), `--seed`,
      `--out`.
- [ ] Ablauf: Runde 0 = BC auf `teacher_rollout`-Daten (beide Topologien
      gemischt); Runden 1…R = `dagger_rollout` mit dem aktuellen Netz,
      neue Daten ans **Aggregat** anhängen, weitertrainieren
      (Cross-Entropy auf die 6 absoluten Richtungen, kein
      Toward-Weighting — der Lehrer ist schon futterorientiert).
- [ ] Nach jeder Runde: `cnn_eval_games` auf Eval-Seeds 10 000–10 099,
      beide Topologien, `max_ticks` 20 000; loggen: Per-Move-Agreement
      (Holdout), `won%`, ⌀Score, ⌀Ticks. **Checkpoint-Auswahl**: höchste
      `min(won%_walls, won%_periodic)`, Tiebreak weniger ⌀Ticks
      (Deployment-Modus-Lehre aus Run 022–025; `best.cnn` laufend
      sichern).
- [ ] **Fehleranalyse**: Partien, die der Student verliert, als
      `(board_seed, tick, zustand)`-Liste dumpen (`--dump-failures`);
      wenn `won%` stagniert, gezielt DAgger-Partien mit diesen Seeds
      nachschieben.
- [ ] **Smoke-Run zuerst** (`--bc-games 10 --dagger-rounds 1
      --dagger-games 5 --epochs-per-round 2`, Ausgabe nach `/tmp`), dann
      der **echte Lauf direkt auf dieser Maschine** (Defaults oben;
      Richtwert < 2 h). Bei Agreement-Plateau < 99 % zuerst
      `--conv-channels 24 24`, dann `16 16 16` probieren (je ein Lauf,
      Ergebnisse in den Report).
- [ ] **Doku**: `docs/training/distill/guide.md` (Skill
      `/training-docs`), Run-Report, Blog-Notizen (Agreement- vs.
      `won%`-Kurve!).
- [ ] **Deployment**: Bestes Netz nach
      `crates/snake-core/assets/cnn/best.cnn`, wenn es den bisherigen
      `ConvNet`-Stand (Run 001: Walls 11.7 / Periodic 8.6) schlägt —
      davon ist auszugehen; `cargo test -p snake-core` (embedded-Tests)
      und `/check` danach.

**Done wenn:** Der beste Checkpoint erreicht das Primärziel (≥ 95 %
`won%` auf 16×12, beide Topologien, greedy) — **oder** drei
Architektur-/DAgger-Varianten sind gelaufen und der beste Stand ist mit
Fehleranalyse im Report dokumentiert. In beiden Fällen weiter mit
Phase B.

## Phase B — AlphaZero-Conv mit Endgame-Curriculum (Pflicht, unabhängig von A)

Ausgeführt **auch wenn Phase A das Primärziel erreicht** — die Frage
„*lernt* RL Perfektion, statt sie zu imitieren?" ist eigenständig
wertvoll. Basis: `python/train_alphazero_conv.py` + `az_conv_selfplay`
(Doku: `docs/training/cnn/guide.md` §4).

- [ ] **Start-State-Sampler** in `snake-core`/`snake-py`:
      `sample_start_state(boundary, width, height, board_seed,
      target_len)` — spielt eine Lehrer-Partie (`CycleSurgeon`) auf
      `board_seed` und gibt den `GameState` beim ersten Tick mit
      Schlangenlänge `target_len` zurück (deterministisch). In
      `az_conv_selfplay` als Option `teacher_start_fraction: f32`
      (CLI `--teacher-starts`, Default 0.5): dieser Anteil der
      Self-Play-Partien startet von einem Lehrer-Zustand mit
      `target_len ~ Uniform{3 … 0.9·Zellen}` (RNG des Self-Play-Seeds),
      Rest startet normal. Value-Target/Return-Berechnung unverändert ab
      dem Startzustand.
- [ ] **Won-Bonus in der Suche**: Im dichten Schritt-Reward der geteilten
      MCTS (`alphazero.rs`) terminales `Won` mit +1.0 belohnen (symmetrisch
      zum Todes-Malus), damit „Brett gefüllt" von „lange überlebt"
      unterscheidbar ist. MLP-AZ-Tests müssen grün bleiben (geteilte
      Suche!).
- [ ] **Smoke-Run** (Parameter wie in `docs/training/cnn/guide.md` §4,
      plus `--teacher-starts 0.5`), Export-Self-Check muss durchlaufen.
- [ ] **Echter Lauf direkt auf dieser Maschine**:
      `--games-per-iter 128 --sims 24 --boundary mixed --max-ticks 2000
      --teacher-starts 0.5 --conv-channels 16 16 16 --head-hidden 24
      --epochs 4 --lr 1e-3 --seed 1 --eval-every 5 --eval-games 12
      --eval-max-ticks 20000 --max-hours 6 --iterations 100000`.
      Als Hintergrundprozess mit Log; alle ~30 min Eval-Kurve prüfen.
- [ ] **Abbruchkriterium** (vorab fixiert): Liegt das Walls-Eval-Mittel
      nach 6 h unter 60, wird **nicht** verlängert — Negativergebnis
      dokumentieren (Lehre aus Run 026/027: Compute ersetzt keinen
      Lernhebel). Optional *ein* Folgelauf mit `--teacher-starts 0.8`,
      falls die Kurve klar steigend abgeschnitten wurde.
- [ ] **Auswertung**: `bench_cnn` (200 Partien, 20 000 Ticks) für
      `best.cnn`; Vergleichstabelle A vs. B vs. Run AZ-001 vs.
      CycleSurgeon (`won%`, ⌀Score, ⌀Ticks, Params, Trainingskosten) im
      Report; Deployment nach `assets/alphazero-cnn/best.cnn` nur, wenn
      besser als Run AZ-001 (Walls 4.1 / Periodic 77.5). WASM-Kosten des
      tieferen Netzes vor dem Einbetten messen (ein Frame-Budget-Check
      im Browser via `/run-web` genügt).
- [ ] **Blog-Notizen**: Wirkung des Curriculums auf die Walls-Schieflage,
      Endgame-`won%`, Vergleich zur Distillation.

**Done wenn:** Der Lauf ist mit vollem Budget (oder erfülltem
Abbruchkriterium) durch, der Report samt Vergleichstabelle existiert, und
das Ergebnis ist deployed **oder** als Negativergebnis dokumentiert.

## Phase C — Gelernte Shortcut-Policy (nur konditional)

**Nur ausführen, wenn weder A noch B das Primärziel erreicht haben.**
Ehrliche Einordnung: Perfektion kommt hier aus der Maske, gelernt wird
die Geschwindigkeit.

- [ ] **Aktionsraum**: Kandidaten = alle per (gemeinsam genutztem)
      `shortcut_is_safe` beweisbar sicheren Sprünge entlang der
      Zyklusordnung des `CycleSurgeon`-Zyklus (+ „Zyklus folgen"). *Jede*
      Policy über diesem Aktionsraum spielt perfekt.
- [ ] **Kleines Scoring-MLP** über Kandidaten-Features
      (Zyklus-Distanzgewinn, Vacate-Slack, Torus-Distanz zum Futter,
      Schlangenlänge/Brettfüllung); Training per ES in `snake-train`
      (vorhandener `evolve`-Kern), Fitness = −Ticks bis `Won`, gemischte
      Topologien. Läuft direkt auf dieser Maschine (Richtwert ≤ 1 h).
- [ ] **Benchmark**: 100 % `won` (per Konstruktion, trotzdem messen) und
      weniger ⌀Ticks als der ungelernte `CycleSurgeon` — sonst lohnt der
      Dropdown-Eintrag nicht (dann nur Report + Blog-Notiz).

**Done wenn:** Eine gelernte Strategie gewinnt jede Partie und ist
messbar schneller als der `CycleSurgeon` — oder Phase C wurde nicht
gebraucht bzw. das Ergebnis ist dokumentiert und verworfen.

---

## Offene Fragen / Risiken

- **Zählt das Safety-Masking?** Alle Netz-Strategien im Repo maskieren
  sofort tödliche Züge — das bleibt der Standard und gilt weiterhin als
  „das Netz spielt". Tiefere Suche (MCTS) zur Inferenzzeit wäre eine
  andere Kategorie und wird für das Primärziel *nicht* verwendet.
- **2-opt-Umkehrkosten (Phase D)**: Segmentumkehr ist O(Segment); der
  Deckel (≤ 32) hält den Tick billig. Falls Relocate allein reicht
  (messen!), 2-opt weglassen — weniger Code schlägt mehr Ops.
- **DAgger-Lehrer im Lockstep (Phase A)**: Der Lehrer ist stateful
  (mutierender Zyklus). Deshalb pro Partie eine eigene Lehrer-Instanz,
  die dieselbe Zustandsfolge sieht wie der Student — *nicht* pro Zustand
  frisch konstruieren (sonst wären Labels inkonsistent teuer/instabil).
- **Fehlerraten-Mathematik**: 95 % Perfect-Rate über ~7 000 Züge heißt
  Per-Zug-Fehler ≲ 7·10⁻⁶ auf dem Spielpfad — erreichbar, weil die
  Lehrer-Policy fast deterministisch ist und Abweichungen selten fatal
  sind (DAgger korrigiert zurück auf den Zyklus). Wenn nicht: Phase C.
- **Generalisierung über Brettgrößen**: Das Conv-Netz ist größenagnostisch,
  der Zyklus brettspezifisch. Primärziel ist 16×12; der Stretch (24×18)
  wird erst nach Erreichen des Primärziels geprüft (ggf. Training auf
  gemischten Größen — eigener, optionaler Lauf).
- **WASM-Budget**: Ein Perfect Game im Browser heißt ~10⁴ Conv-Forwards;
  bei den kleinen Netzen (~3–10 k Params) unkritisch erwartet, aber vor
  dem Einbetten des tieferen Phase-B-Netzes messen.
- **Ungerade Zeilenzahl**: Serpentinen-Zyklus braucht gerade Zeilenzahl
  (alle Presets kompatibel). `CycleSurgeon` erbt die Einschränkung; die
  destillierten Netze erben sie implizit über die Trainingsdaten — freie
  Feldgrößen bleiben Best-Effort ohne Perfektionsversprechen.
- **Lehrer-Deckelung**: Für Lehrer-Rollouts (Phase A) und den
  Start-State-Sampler (Phase B) Tick-Limit 20 000 setzen, damit nur
  `Won`-Partien als Material dienen.
