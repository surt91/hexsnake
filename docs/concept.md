# HexSnake — Konzept

Snake auf einem hexagonalen Gitter, spielbar im Browser (Rust → WASM, egui),
mit klassischen Regeln, umschaltbaren Randbedingungen, Highscore und einem
Autopiloten mit mehreren austauschbaren Strategien.

## 1. Spielidee & Grundregeln

- Die Schlange bewegt sich auf einem Hexgitter und kann in **6 Richtungen** laufen.
- Klassische Regeln:
  - Futter fressen → Schlange wird um ein Segment länger, Punkte steigen.
  - Beißt sich die Schlange selbst → Game Over.
- **Randbedingungen umschaltbar** (im Menü, vor Spielstart):
  - **Wände**: Kollision mit dem Rand beendet das Spiel.
  - **Periodisch** (Torus): Die Schlange tritt am gegenüberliegenden Rand wieder ein.
- 180°-Umkehr ist verboten (die der aktuellen Bewegungsrichtung entgegengesetzte
  Richtung wird ignoriert), wie beim klassischen Snake.

## 2. Hex-Geometrie & Steuerung

**Orientierung: Flat-Top-Hexagons** (flache Seite oben). Damit sind die sechs
Nachbarrichtungen genau Nord, Süd, Nordost, Südost, Nordwest, Südwest — das
passt perfekt auf die QWEASD-Belegung:

```
        W (N)
  Q (NW)   E (NE)
  A (SW)   D (SE)
        S (S)
```

| Taste | Richtung  | Gegenrichtung (verboten) |
|-------|-----------|--------------------------|
| `W`   | Nord      | Süd                      |
| `E`   | Nordost   | Südwest                  |
| `D`   | Südost    | Nordwest                 |
| `S`   | Süd       | Nord                     |
| `A`   | Südwest   | Nordost                  |
| `Q`   | Nordwest  | Südost                   |

**Koordinatensystem**: Intern axiale Koordinaten `(q, r)` für Distanzen und
Nachbarschaftslogik; das Spielfeld selbst ist ein "Rechteck" in
Offset-Koordinaten `(col, row)`, damit periodische Randbedingungen ein
einfaches `mod Breite / mod Höhe` sind. Konvertierung axial ↔ offset ist
Standard (vgl. Red Blob Games, Hexagonal Grids).

Eingaben werden in einer kleinen **Input-Queue** (2–3 Einträge) gepuffert,
damit schnelle Doppel-Eingaben („erst NE, dann SE") nicht verloren gehen.

## 3. Autopilot-Strategien

Der Autopilot ist eine austauschbare Strategie hinter einem gemeinsamen Trait:

```rust
trait Strategy {
    fn next_move(&mut self, state: &GameState) -> Direction;
}
```

Alle Strategien laufen deterministisch auf dem reinen Spielzustand (kein
UI-Zugriff), sind damit testbar und können im Battle-Modus gegeneinander
antreten. Vorschlag für **7 Strategien**, von dumm bis clever:

### 3.1 Chaos-Walker (Zufalls-Baseline)
Wählt gleichverteilt zufällig unter den Zügen, die nicht sofort tödlich sind.
Nützlich als Baseline für Vergleiche und überraschend unterhaltsam anzusehen.

### 3.2 Greedy
Wählt den Zug, der die Hex-Distanz zum Futter am stärksten verringert
(bei periodischen Rändern: Distanz auf dem Torus, d. h. Minimum über die
Wrap-Varianten). Sofort tödliche Züge werden ausgeschlossen. Schnell, einfach,
aber sperrt sich gern selbst ein — der klassische „naive Spieler".

### 3.3 Pfadplaner (A* + Tail-Check)
1. Berechnet per A* den kürzesten sicheren Pfad zum Futter.
2. **Survival-Check**: Simuliert das Abfahren dieses Pfads und prüft, ob die
   Schlange danach ihr eigenes Schwanzende noch erreichen kann (Schwanz
   erreichbar ⇒ sie kann nie endgültig eingesperrt sein).
3. Falls kein sicherer Pfad existiert: **Tail-Chasing** — folgt dem eigenen
   Schwanz und wartet auf eine bessere Gelegenheit.

Sehr starke, klassische Snake-KI; gut als „Standard-Autopilot".

### 3.4 Raumgreifer (Flood-Fill-Survivalist)
Bewertet jeden möglichen Zug danach, wie viele freie Felder danach per
Flood-Fill noch erreichbar sind, und maximiert diesen Freiraum. Futter wird
nur opportunistisch genommen (Tiebreaker: geringere Futterdistanz). Spielt
extrem defensiv, überlebt lange, scored langsam — ein interessanter Kontrast
zum Pfadplaner.

### 3.5 Hamilton-Fahrer (mit Abkürzungen)
Vorberechneter Hamilton-Zyklus über das Spielfeld (auf dem Hexgitter z. B. als
Serpentinen-Muster aus NE/SE-Zickzack-Zeilen konstruierbar; erfordert passende
Feldmaße, was der Levelvalidator sicherstellt). Die Schlange folgt dem Zyklus
und kann **prinzipiell nie sterben**. Damit es nicht ewig dauert: erlaubte
**Shortcuts** entlang der Zyklusordnung, wenn der Sprung nachweislich kein
Segment überspringt, das noch im Weg liegt. Auf dem Torus besonders elegant.
Das ist die „Perfektionist"-Strategie: langsam, aber füllt das Brett komplett.

### 3.6 Monte-Carlo-Lookahead
Für jeden legalen Zug werden N zufällige (oder greedy-geführte) Rollouts über
einen Horizont von k Ticks simuliert; bewertet wird Überleben + gefressenes
Futter + verbleibender Freiraum. Budget pro Tick begrenzt, damit es auch in
WASM flüssig bleibt. Spielt „menschlich riskant" und ist durch die
Zufallskomponente bei jedem Lauf anders.

### 3.7 Neuronales Netz
Kleines MLP (zwei Hidden-Layer reichen), Inputs als **Sensorstrahlen in die
6 Hex-Richtungen** (je Richtung: Distanz zur Wand/zum eigenen Körper/zum
Futter) plus aktuelle Richtung und Längen-Feature; Output: Score je Richtung.
Training **offline** als natives Rust-Binary gegen die Game-Engine — Vorschlag:
**Evolutionsstrategie / genetischer Algorithmus** (robust, kein
Gradienten-Setup nötig; Population spielt Partien, Fitness = Score +
Überlebenszeit). Die fertigen Gewichte werden als Asset eingebettet; die
Inferenz ist ein paar Matrixmultiplikationen in purem Rust und läuft problemlos
in WASM. Mehrere Checkpoints („Generation 10 / 100 / 1000") wären ein nettes
Gimmick, um den Lernfortschritt anzuschauen.

### 3.8 Weitere Lernverfahren (Ausbaustufe)

Aufbauend auf dem GA/ES-Netz ist eine eigene ML-Ausbaustufe geplant, in der
mehrere Trainingsverfahren als wählbare Autopiloten nebeneinanderstehen —
eine Treppe von „Evolution ohne Gradienten" bis „modernes RL":

- **NEAT**: Neuroevolution, die auch die Netztopologie evolviert. Bleibt im
  Rust-Track (kein Gradient nötig); die entstehenden Minimalnetze sind im
  Debug-Overlay schön anzuschauen. **CMA-ES** als Drop-in-Upgrade des
  Evolutions-Optimierers für den direkten Vergleich.
- **Behavior Cloning**: Der A*-Pfadplaner generiert gelabelte
  Zustand→Zug-Paare, ein Netz imitiert ihn per Supervised Learning. Billig
  zu trainieren, spannender Vergleich (Imitator vs. Original vs. RL) und
  brauchbarer Warmstart für PPO.
- **DQN und PPO**: klassisches Deep RL via `stable-baselines3`. Dafür wird
  `snake-core` per **PyO3/maturin** nach Python gebunden — Training in
  Python gegen die echte Rust-Engine (kein Drift), Gewichts-Export in ein
  simples Format, Inferenz weiterhin pur in Rust/WASM. Optional: Brett als
  Gitter-Tensor + CNN-Input statt der 6 Sensorstrahlen.
- **AlphaZero-light** (optional, anspruchsvollste Stufe): Policy/Value-Netz
  ersetzt die Zufalls-Rollouts des Monte-Carlo-Lookahead, trainiert per
  Self-Play — große Synergie mit der vorhandenen MCTS-Strategie.

## 4. Feature- & Gimmick-Vorschläge

Geordnet nach Aufwand/Nutzen; (★) = Empfehlung für die erste Ausbaustufe.

### Gameplay
- **Spezialfutter** (★): goldener Apfel (Bonuspunkte, despawnt nach Timeout),
  fauler Apfel (Schlange schrumpft / Punktabzug), Tempo-Frucht (kurzzeitig
  schneller, Punkte-Multiplikator als Risikoprämie).
- **Steigende Geschwindigkeit** (★): Ticks werden mit der Länge schneller;
  alternativ wählbare feste Schwierigkeitsstufen.
- **Combo-System**: schnelles Fressen in Folge erhöht einen Multiplikator.
- **Hindernisse / Level**: vordefinierte Karten mit Felswänden; Levelvalidator
  stellt Erreichbarkeit sicher.
- **Portale**: zwei verbundene Hexfelder, die Schlange teleportiert sich —
  besonders lustig in Kombination mit den KI-Strategien.

### KI & Nerd-Appeal
- **Debug-Overlay** (★): Visualisierung der KI-Entscheidung — geplanter
  A*-Pfad, Flood-Fill-Heatmap, NN-Output je Richtung. Macht die Strategien
  erlebbar und hilft beim Entwickeln.
- **Battle-/Turniermodus**: zwei oder mehr KI-Schlangen auf einem Brett bzw.
  Strategien im Schnelldurchlauf-Benchmark gegeneinander (Tabelle: ⌀-Score,
  ⌀-Überlebenszeit).
- **Schaumodus**: Autopilot als Bildschirmschoner mit hoher Tickrate.

### Meta & Komfort
- **Deterministische Seeds + Replay** (★): Spielverlauf = Seed + Inputliste;
  ermöglicht Replays, Ghost-Rennen gegen den eigenen Highscore-Lauf und
  **Daily Challenge** (alle spielen denselben Seed).
- **Statistiken**: Spiele, ⌀-Länge, Bestwerte je Modus/Randbedingung.
- **Achievements**: „Brett halb gefüllt", „10× über den Rand gewrappt", …
- **Themes**: hell/dunkel, Farbschemata, dezente Animationen (Fress-Pulse).
- **Touch-Steuerung**: virtuelles Hex-Pad für Mobilgeräte; alternativ Swipe
  in 6 Richtungen.
- **Sound**: kurze Effekte (fressen, Game Over) via Web Audio, abschaltbar.

### Server-Features (alle optional, Spiel bleibt offline voll funktionsfähig)
- **Globaler Highscore** (★): Tabelle je Modus (Randbedingung × Feldgröße ×
  Geschwindigkeit), Name frei wählbar. Anti-Cheat light: Client sendet
  Seed + Inputliste, Server re-simuliert den Lauf zur Verifikation.
- **Daily-Challenge-Leaderboard**: Tagesseed kommt vom Server, Fallback auf
  datumsbasierten lokalen Seed.
- **Multiplayer** (spätere Ausbaustufe): 2–4 Schlangen via WebSocket,
  Server-autoritativ; Kollision mit fremder Schlange = Game Over für den
  Beißenden.

## 5. Technik-Architektur

**Cargo-Workspace** mit strikter Trennung von Logik und UI:

```
hexsnake/
├── crates/
│   ├── snake-core/     # Spiellogik: Hexgitter, GameState, Tick, Strategien.
│   │                   # Keine UI-Abhängigkeiten, deterministisch (Seed-RNG),
│   │                   # vollständig testbar, kompiliert nach WASM und nativ.
│   ├── snake-app/      # egui/eframe-Frontend. Nativ (Dev-Komfort) und
│   │                   # wasm32-unknown-unknown via trunk für den Browser.
│   ├── snake-train/    # Natives Binary: NN-Training (GA/ES) gegen snake-core.
│   └── snake-server/   # Optional: axum, REST-Highscore, später WebSocket.
└── ...
```

- **Rendering**: egui `Painter` mit vorberechneten Hex-Polygonen; bei Bedarf
  später Mesh-Caching. Feldgrößen bis ~40×30 sind damit unkritisch.
- **Game-Loop**: fester Tick (z. B. 6–12 Ticks/s, geschwindigkeitsabhängig),
  entkoppelt vom Render-Frame; `ctx.request_repaint_after(...)`.
- **Persistenz**: lokaler Highscore + Einstellungen über die
  eframe-`Storage`-API (im Browser localStorage, nativ Datei) — ein Codepfad
  für beide Targets.
- **Server-Anbindung**: HTTP via `ehttp` (WASM-tauglich, async-frei).
  Fire-and-forget mit Timeout; ist der Server nicht erreichbar, merkt das
  Spiel sich den Lauf lokal und zeigt nur die lokale Tabelle. Kein Feature
  blockiert auf den Server.
- **Determinismus**: eigener seedbarer RNG (`rand` mit festem Algorithmus,
  z. B. `Pcg32`) im Core — Voraussetzung für Replays, Daily Challenge und
  Server-Verifikation.

## 6. Getroffene Entscheidungen

- **Feldgröße — Presets + frei**: Drei Presets (Klein 16×12, Mittel 24×18,
  Groß 32×24) mit globalen Leaderboards; frei wählbare Größen sind zusätzlich
  möglich, bekommen aber nur lokale Highscores.
- **Highscore-Identität — Name, Keypair-ready**: Vorerst nur ein frei
  wählbarer Name. Die Server-API enthält aber von Anfang an ein optionales
  Signaturfeld, sodass ein lokales Keypair (Schutz vor Namens-Nachahmung)
  später ohne Datenmigration nachgerüstet werden kann.
- **Hamilton-Kompatibilität — Presets kompatibel**: Alle Presets werden so
  gewählt, dass die Hamilton-Zyklus-Konstruktion funktioniert (per
  Generator-Test abgesichert). Nur bei frei gewählten, inkompatiblen Maßen
  wird die Strategie ausgegraut, mit erklärendem Tooltip.
- **NN-Training — Rust zuerst, Python-RL als Ausbaustufe**: Das erste Netz
  wird mit GA/ES in purem Rust trainiert (`snake-train`). Danach folgt eine
  eigene ML-Phase mit PyO3-Bindings und weiteren Verfahren (NEAT, Behavior
  Cloning, DQN, PPO, optional AlphaZero-light), siehe Abschnitt 3.8 — Ziel
  ist ein Dropdown, in dem man die Verfahren gegeneinander antreten lassen
  kann.
