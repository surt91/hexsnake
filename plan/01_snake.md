# Plan 01 — HexSnake: Umsetzung

Basiert auf [`docs/concept.md`](../docs/concept.md). Die Phasen sind so
geschnitten, dass nach jeder Phase etwas Lauffähiges existiert. Server und
NN-Training sind bewusst spät — die ersten fünf Phasen ergeben bereits ein
vollständiges Offline-Spiel mit Autopilot.

## Phase 0 — Projekt-Setup

- [x] Cargo-Workspace anlegen: `crates/snake-core`, `crates/snake-app`
      (`snake-train`, `snake-server` folgen später).
- [x] `snake-app` als eframe-Template: läuft nativ (`cargo run`) und im
      Browser (`trunk serve`, Target `wasm32-unknown-unknown`).
- [x] Tooling: `rustfmt`, `clippy`, `index.html` für trunk, README mit
      Build-Anleitung.
- [x] `git init` + erster Commit.

**Done wenn:** Ein leeres egui-Fenster läuft nativ und im Browser.

## Phase 1 — Core: Hexgitter & Spiellogik (reine Logik, keine UI)

- [x] Koordinaten: axial `(q, r)` + Offset `(col, row)` mit Konvertierung;
      `Direction`-Enum (N, NE, SE, S, SW, NW) mit `opposite()` und
      `neighbor(coord, dir)`.
- [x] `BoundaryMode { Walls, Periodic }`: Nachbarberechnung wrappt auf dem
      Offset-Rechteck (mod Breite/Höhe) bzw. liefert „Wand".
- [x] Hex-Distanz, bei `Periodic` als Torus-Distanz (Minimum über
      Wrap-Varianten) — wird von den Strategien gebraucht.
- [x] `GameState`: Schlange (VecDeque), Richtung, Futterposition, Score,
      Tick-Zähler, seedbarer RNG (`Pcg32`).
- [x] `tick(&mut self, input: Option<Direction>)`: Richtungswechsel
      (180°-Verbot), Bewegung, Fressen/Wachsen, Futter-Respawn auf freiem
      Feld, Kollisionserkennung → `GameOver`.
- [x] Input-Queue (2–3 gepufferte Richtungen) als Teil des Core.
- [x] **Tests**: Koordinaten-Roundtrips, Wrap-Verhalten an allen Rändern,
      Wachstum, Selbstbiss, 180°-Verbot, Determinismus (gleicher Seed +
      gleiche Inputs ⇒ identischer Verlauf).

**Done wenn:** Eine Partie ist headless per Unit-Test von Start bis Game Over
durchspielbar.

## Phase 2 — Spielbares Frontend

- [x] Hex-Rendering mit egui-`Painter` (Flat-Top, Polygone aus
      Offset-Koordinaten), Schlange/Kopf/Futter klar unterscheidbar.
- [x] Fester Game-Tick entkoppelt vom Frame
      (`request_repaint_after`), QWEASD-Input → Input-Queue.
- [x] Spielzustände: Startmenü → läuft → Pause (`Space`/`P`) → Game Over →
      Neustart.
- [x] Menü-Optionen: Randbedingung (Wände/Periodisch), Feldgröße als Preset
      (Klein 16×12 / Mittel 24×18 / Groß 32×24, alle Hamilton-kompatibel)
      oder frei wählbar, Startgeschwindigkeit.
- [x] Visuelles Feedback für periodischen Modus (z. B. Randmarkierung statt
      Mauer-Optik).
- [x] Debug-Feature: Seed per URL-Parameter (`?seed=42`, im WASM-Build via
      Query-String, nativ via CLI-Arg) — macht Läufe reproduzierbar für
      Browser-Screenshots und Bug-Reports (vgl. Skill `/test-debug`).
- [x] **GitHub-Pages-Deployment**: GitHub-Actions-Workflow, der bei Push auf
      `main` den Release-Build erzeugt und veröffentlicht
      (`trunk build --release --public-url /hexsnake/` — der Repo-Name als
      Subpfad ist Pflicht, sonst laden WASM/JS-Assets nicht; Deployment via
      `actions/deploy-pages`, wasm32-Target + trunk im Workflow
      installieren/cachen).
      *Deployment ist live: <https://surt91.github.io/hexsnake/>
      (verifiziert 2026-06-13).*

**Done wenn:** HexSnake ist im Browser mit QWEASD spielbar, beide
Randbedingungen funktionieren sichtbar korrekt, und das Spiel ist öffentlich
über GitHub Pages erreichbar.

## Phase 3 — Highscore (lokal)

- [x] Score-Anzeige im Spiel; steigende Geschwindigkeit mit der Länge.
- [x] Lokale Highscore-Tabelle, **getrennt je Modus**
      (Randbedingung × Feldgröße × Geschwindigkeit), Top 10 mit Name + Datum.
      Frei gewählte Feldgrößen bekommen nur lokale Tabellen (global später
      nur Presets).
- [x] Persistenz über eframe-`Storage` (Browser: localStorage; nativ: Datei),
      inkl. zuletzt gewählter Einstellungen.
- [x] Namenseingabe bei neuem Highscore.

**Done wenn:** Highscores überleben einen Browser-Reload.

## Phase 4 — Autopilot-Framework + erste Strategien

- [x] `Strategy`-Trait in `snake-core`; Autopilot im UI zuschaltbar
      (Dropdown), jederzeit per Tastendruck an/aus (Mensch übernimmt).
- [x] **Chaos-Walker**: zufällig unter nicht sofort tödlichen Zügen.
- [x] **Greedy**: Hex-/Torus-Distanz zum Futter minimieren.
- [x] **Pfadplaner**: A* zum Futter + Survival-Check (Schwanz nach
      simuliertem Pfad erreichbar?), sonst Tail-Chasing.
- [x] Benchmark-Harness in `snake-core` (headless, N Partien pro Strategie,
      ⌀-Score/⌀-Überlebenszeit) — als Test/Beispiel-Binary.
- [x] Autopilot-Läufe werden vom Highscore ausgenommen (oder eigene Tabelle).

**Done wenn:** Pfadplaner spielt sichtbar gute Partien in beiden Randmodi;
Benchmark zeigt Chaos < Greedy < Pfadplaner.

## Phase 4b — Bedienkomfort & Seed-Darstellung (Nachtrag)

- [x] **In-Game-Steuerung per Maus**: Autopilot-Strategie während des
      Spiels per Dropdown wechselbar (nicht nur an/aus via `T`), Tempo
      (Basisgeschwindigkeit) ebenfalls mitten im Spiel umstellbar — beides
      mausbedienbar im HUD. **Auto-Pause**, solange das Dropdown geöffnet
      ist (Aufklappen pausiert, Schließen setzt fort; eine manuell gesetzte
      Pause bleibt davon unberührt).
- [x] Tempowechsel während einer Partie: Der Lauf wird in der Tabelle des
      **langsamsten verwendeten Tempos** gewertet — so bringt
      zwischenzeitliches Hochschalten keinen Vorteil in einer schnelleren
      Tabelle, und ein Lauf wird nie unfair entwertet. Die Autopilot-Regel
      bleibt unverändert: Sobald der Autopilot mindestens einen Tick
      gesteuert hat, ist der Lauf komplett vom Highscore ausgeschlossen.
- [x] **Kompakte Seeds**: Seeds auf 32 Bit reduzieren **und** URL-safe
      Base64-kodiert darstellen (6 Zeichen statt bis zu 20
      Dezimalstellen). HUD und Menü zeigen nur noch die kompakte Form;
      `?seed=`/`--seed` akzeptiert beide Schreibweisen (dezimal und
      Base64). Zufalls-Seeds werden entsprechend nur noch aus 32 Bit
      gezogen. Encoding/Decoding mit Unit-Tests (Roundtrip).

**Done wenn:** Strategie- und Tempowechsel sind mitten im Spiel per Maus
möglich (inkl. Auto-Pause beim offenen Dropdown), und der Seed erscheint
überall als kurzer Base64-String, der per URL wieder ladbar ist.

## Phase 5 — Weitere Strategien

- [x] **Raumgreifer**: Flood-Fill-Bewertung pro Zug, Futter als Tiebreaker.
- [x] **Hamilton-Fahrer**: Serpentinen-Hamilton-Zyklus konstruieren
      (Generator + Test, dass jeder Zyklus gültig ist); Test stellt sicher,
      dass **alle drei Presets kompatibel** sind. Shortcut-Logik entlang der
      Zyklusordnung; bei frei gewählten, inkompatiblen Maßen wird die
      Strategie ausgegraut (Tooltip erklärt warum).
      *(UI-Ausgrauen folgt im Overlay/Dropdown-Punkt dieser Phase.)*
- [x] **Monte-Carlo-Lookahead**: Rollouts mit Tick-Budget, Parameter
      (N, Horizont) als Konstanten mit sinnvollen Defaults.
- [x] **Debug-Overlay**: A*-Pfad, Flood-Fill-Heatmap, MC-Bewertung je
      Richtung einblendbar (Taste `O`; zeigt auch den Hamilton-Zyklus).

**Done wenn:** Fünf+ Strategien wählbar, Overlay zeigt nachvollziehbar, was
die KI „denkt".

## Phase 6 — Skins, Mobile & Statistik

*(War ursprünglich Phase 7; mit dem Neuronalen Netz getauscht, damit die
rechenintensive Trainingsphase ganz ans Ende rückt — Entscheidung vom
2026-06-13.)*

*(Spezialfutter und Replay/Ghost wurden bewusst nach „Spätere
Ausbaustufen" verschoben — Entscheidung vom 2026-06-12.)*

- [x] **Theme-/Skin-System**: Theme-Abstraktion als reine Daten + Zeichenstil
      (Farben, Kopf-/Segment-Form, Futter-Darstellung) — kein Zugriff auf
      die Spiellogik, prozedural gezeichnet (keine Sprite-Assets). Auswahl
      im Menü, Wahl wird persistiert.
- [x] Skins umsetzen: **Klassisch** (cleane Geometrie), **Honigwabe**
      (Wabenoptik, Raupe/Bienen, Honigtropfen als Futter), **Retro-LCD**
      (monochromes Nokia-Grün), **Neon** (dunkler Grund, Glow via
      halbtransparente Outlines), **Naturalistisch** (Kopf mit Augen, die
      Richtung Futter blicken; zum Schwanz schmaler werdende Segmente,
      Apfel als Futter).
- [x] **Pfad-Rendering („Schlängeln sichtbar")**: Mindestens ein Skin
      zeichnet die Schlange als durchgehenden Körper von Kante zu Kante —
      ein Band durch die Zellzentren mit abgerundeten Gelenken, sodass der
      Verlauf der Windungen erkennbar bleibt und nicht nur, *welche* Felder
      belegt sind (sonst sieht man bei langer Schlange v. a. ein gefülltes
      Gitter). Dafür liefert die Render-Schicht je Segment die Ein-/
      Austrittsrichtung (Vorgänger/Nachfolger); am Torus-Wrap bricht das
      Band ab und setzt am gegenüberliegenden Rand fort. Naheliegende
      Kandidaten: **Naturalistisch** (Körperband, das zum Schwanz schmaler
      wird) und **Neon** (Glow-Linie).
- [x] **Farbenblind-sichere Palette** als Pflicht-Theme: Schlange und Futter
      unterscheiden sich auch über Form, nicht nur Farbe.
- [x] Fress-/Game-Over-Effekte (theme-übergreifend).
- [x] Touch-Steuerung (virtuelles Hex-Pad) für Mobile.
- [x] Statistik-Panel (Spiele, ⌀-Länge, Bestwerte).

**Done wenn:** Alle Skins inkl. Farbenblind-Theme sind wählbar und
überleben einen Reload, Fress-/Game-Over-Effekte sind sichtbar, das Spiel
ist per Touch auf einem Mobilgerät steuerbar und das Statistik-Panel zeigt
plausible Werte.


## Phase 7 — Neuronales Netz

*(War ursprünglich Phase 6, jetzt nach den Skins. **Trainings-Politik**:
Echte Trainingsläufe sind rechenintensiv und werden vom Nutzer auf einem
stärkeren Rechner ausgeführt — bei der Implementierung läuft nur ein
minimaler Smoke-Run zu Testzwecken. Für jede Strategie, die einen
Trainingslauf braucht, gehört eine Anleitung nach
`docs/training/<name>.md` (siehe Skill `/training-docs`): von der
Installation der Abhängigkeiten über den konkreten Aufruf bis zu
Hyperparameter-Wahl und Auswertung.)*

- [x] Sensor-Featurevektor in `snake-core` (6 Richtungsstrahlen: Distanz zu
      Hindernis/Körper/Futter, + Richtung/Länge).
- [x] Mini-MLP (Forward-Pass pur in Rust, keine externen ML-Deps);
      Gewichts-(De)Serialisierung in einem simplen, dokumentierten Format —
      dasselbe Format nutzt später auch der Python-Export (Phase 9).
- [x] `snake-train`: Evolutionsstrategie/GA, Fitness = Score +
      Überlebenszeit, parallelisiert (rayon), Checkpoints speichern.
- [x] **Smoke-Training** (klein, nur zur Verifikation der Pipeline) +
      Mechanik zum Einbetten von Gewichts-Assets; Strategie
      „Neural Net (Gen X)" im Dropdown, Gewichte austauschbar, sobald der
      Nutzer den echten Lauf gemacht hat.
- [x] **Trainings-Anleitung** `docs/training/neural-net-ga.md` schreiben
      (Skill `/training-docs`): Voraussetzungen, Befehle, Hyperparameter,
      erwartete Laufzeit, Auswertung, Einbetten der Checkpoints.
- [x] NN in den Benchmark aufnehmen.

**Done wenn:** Die Trainings-Pipeline läuft end-to-end (Smoke-Run), das
Smoke-NN ist im Browser wählbar, und die Anleitung erlaubt es, den echten
Trainingslauf ohne weitere Rückfragen auf einem anderen Rechner
durchzuführen. (Das Kriterium „NN schlägt Greedy deutlich" wandert zum
echten Trainingslauf des Nutzers.)

## Phase 8 — Optionaler Server

- [x] `snake-server`: axum, SQLite; Endpoints `GET/POST /highscores/{mode}`.
      Globale Leaderboards nur für die drei Presets; Identität ist ein frei
      wählbarer Name, das Submit-Schema enthält aber von Anfang an ein
      **optionales Signaturfeld** (Keypair-Nachrüstung später ohne
      Migration).
- [x] Verifikation: Client sendet Seed + Inputliste, Server re-simuliert mit
      `snake-core` (gleiche Crate ⇒ gleiches Verhalten) und akzeptiert nur
      konsistente Läufe.
- [x] **Härtung der öffentlichen Endpoints**:
      - Body-Size-Limit (axum `DefaultBodyLimit`) und Eingabevalidierung
        (Namenslänge, erlaubte Zeichen, plausible Feld-/Modus-Werte).
      - Rate Limiting pro IP auf dem POST-Endpoint (z. B. `tower-governor`);
        hinter Proxy/Tunnel die Client-IP aus `X-Forwarded-For` nehmen,
        konfigurierbar.
      - Re-Simulation deckeln: harte Obergrenze für Inputlistenlänge und
        Tick-Zahl, Verifikation mit Concurrency-Limit (Semaphore), damit
        teure Anfragen den Server nicht auslasten können.
      - Tests: überlange Namen, überlange Inputlisten und inkonsistente
        Läufe werden mit 4xx abgelehnt.
- [x] Client: `ehttp`-Anbindung mit Timeout; nicht erreichbar ⇒ stilles
      Fallback auf lokale Tabelle, ausstehende Läufe werden lokal gemerkt
      und später nachgereicht. UI zeigt lokale und globale Tabelle.
- [x] Daily Challenge: Tagesseed vom Server, lokaler datumsbasierter
      Fallback; eigenes Leaderboard.
- [x] **All-in-one-Dockerfile**: Multi-Stage-Build (Stage 1: trunk-Release-
      Build des WASM-Frontends; Stage 2: cargo-Build des Servers; Runtime-
      Stage: schlankes Image). Der axum-Server liefert neben der API auch
      die statischen Dateien aus (`tower-http` `ServeDir`) — damit ist das
      komplette Spiel inkl. Highscore-Server als ein Container hostbar.
      SQLite-Datei auf einem Volume (`/data`), Pfad per Env-Var.
      Container gehärtet: läuft als Non-Root-User, Root-Filesystem
      read-only (beschreibbar nur `/data`), keine zusätzlichen
      Capabilities; Beispiel-`docker-compose.yml` mit diesen Optionen
      beilegen.
- [x] Client-Konfiguration für beide Hosting-Varianten: API-Aufrufe gehen
      per Default an **relative URLs** (same-origin — deckt die Docker-
      Variante ohne CORS ab); für den GitHub-Pages-Build wird die
      Server-URL zur Buildzeit/per Konfiguration gesetzt. GitHub Pages
      bleibt das primäre Hosting.
- [x] Deployment-Notizen (`docs/deployment.md`) mit drei Varianten:
      - **Docker auf VPS**: Volume für SQLite, Reverse Proxy (Caddy/Traefik)
        für automatisches HTTPS.
      - **Heimserver**: empfohlen via **Cloudflare Tunnel** (`cloudflared`
        als Sidecar-Container — kein offener Port, Heim-IP bleibt verborgen,
        funktioniert auch hinter DS-Lite/CGNAT). Falls doch Portforwarding:
        nur 443 (+80) weiterleiten, Host in eigenes VLAN/DMZ isolieren,
        SSH nie exponieren (Administration nur via LAN/VPN), automatische
        Updates (unattended-upgrades, regelmäßige Image-Rebuilds), Backup
        des SQLite-Volumes.
      - **GitHub Pages + externer API-Server**: CORS muss die Pages-Origin
        erlauben, und der API-Server braucht **HTTPS** — Pages läuft auf
        HTTPS, Mixed Content zu einem HTTP-Server wird vom Browser
        blockiert.

**Done wenn:** Globaler Highscore funktioniert; mit gezogenem Netzwerkstecker
verhält sich das Spiel exakt wie vorher.

## Phase 9 — ML-Ausbaustufe: weitere Lernverfahren

Ziel: ein Strategie-Dropdown mit GA/ES, NEAT, DQN und PPO (inkl.
CNN-Observation), die im Benchmark gegeneinander antreten (vgl. Konzept §3.8).
Behavior Cloning wurde bewusst weggelassen.

- [ ] **NEAT** in `snake-train` (Rust-Track, kein Gradient nötig) —
      Netzstruktur wächst während des Trainings.
- [ ] **PyO3-Bindings**: `snake-core` via maturin als Python-Modul
      (Gym-artiges Env-Interface: `reset`/`step`/Observation), Python-Setup
      unter `python/` (uv/venv, dokumentiert). Voraussetzung für DQN & PPO.
- [ ] **DQN und PPO** mit `stable-baselines3` gegen das PyO3-Env;
      Reward-Shaping (Futter, Überleben, Freiraum) dokumentieren. Export der
      Policy-Netze ins Rust-Gewichtsformat (Inferenz bleibt pur Rust/WASM).
- [ ] **CNN-Input** (ganzes Brett als Gitter-Tensor) als alternative
      Observation für DQN/PPO, Vergleich gegen Sensorstrahlen-Variante.
- [ ] **AlphaZero-light** — Policy/Value-Netz ersetzt die Zufalls-Rollouts
      des Monte-Carlo-Lookahead, Training per Self-Play.
- [ ] Alle Verfahren als Dropdown-Einträge + Aufnahme in den
      Benchmark-Harness (Vergleichstabelle ⌀-Score/⌀-Überlebenszeit).
- [ ] Für **jedes** Verfahren mit Trainingslauf eine Anleitung
      `docs/training/<name>.md` (Skill `/training-docs`); der Agent führt
      nur Smoke-Runs aus, echte Läufe macht der Nutzer auf stärkerer
      Hardware.

**Done wenn:** GA/ES, NEAT, DQN und PPO sind im Browser wählbar und
der Benchmark zeigt eine Vergleichstabelle.

## Spätere Ausbaustufen (bewusst nicht geplant)

- **Spezialfutter**: goldener Apfel (Bonuspunkte, despawnt nach Timeout),
  fauler Apfel, Tempo-Frucht — im Core inkl. Tests, dann UI; per Menü
  abschaltbar („Klassisch-Modus", bleibt Default für Highscores). Die
  Farbenblind-Anforderung (Unterscheidung über Form) gilt dann auch hier.
  *(Aus Phase 7 verschoben, 2026-06-12.)*
- **Replay & Ghost**: Seed + Inputliste aufzeichnen, Replay-Player im UI,
  Ghost-Anzeige des besten eigenen Laufs. Hinweis: Die Aufzeichnung von
  Seed + Inputliste selbst kommt unabhängig davon mit Phase 8 (Grundlage
  der Server-Verifikation) — hier geht es nur um Player-UI und Ghost.
  *(Aus Phase 7 verschoben, 2026-06-12.)*
- Multiplayer via WebSocket (server-autoritativ, 2–4 Schlangen).
- KI-Battle-/Turniermodus im UI, Hindernis-Level, Portale, Achievements,
  Sound.
- Freischaltbare Themes als Achievement-Belohnung (z. B. „Neon ab
  50 Punkten auf Groß") — setzt das Achievement-System voraus.

## Risiken & Stolpersteine

- **Hex + Torus**: Wrap in Offset-Koordinaten ist einfach, aber
  Distanz/Pfadsuche müssen konsequent die Torus-Metrik nutzen — früh testen
  (Phase 1), sonst spielen die Strategien im periodischen Modus schlecht.
- **Hamilton auf Hex**: Zyklus-Konstruktion hat Paritätsbedingungen an die
  Feldmaße; deshalb Generator + Validierungstest statt Ad-hoc-Konstruktion.
- **WASM-Budget**: Monte-Carlo und Flood-Fill pro Tick deckeln (Zeit-/
  Simulationsbudget), damit auch schwache Geräte flüssig bleiben.
- **Determinismus**: Keine HashMap-Iterationsreihenfolge oder Float-Akrobatik
  im Core verwenden, sonst bricht die Server-Verifikation der Replays.
- **Rust↔Python-Gewichtsformat** (Phase 9): Export/Import mit einem
  Roundtrip-Test absichern (Python-Forward-Pass und Rust-Inferenz liefern
  auf Testinputs identische Outputs), sonst schleichen sich stille
  Transponierungs-/Layout-Fehler ein.
