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
      *Hinweis: Workflow liegt bereit und der Release-Build ist lokal
      verifiziert; es existiert aber noch kein GitHub-Remote — Repo anlegen,
      `git remote add` + Push und Pages aktivieren (Source: GitHub Actions)
      muss einmalig manuell passieren.*

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

- [ ] `Strategy`-Trait in `snake-core`; Autopilot im UI zuschaltbar
      (Dropdown), jederzeit per Tastendruck an/aus (Mensch übernimmt).
- [ ] **Chaos-Walker**: zufällig unter nicht sofort tödlichen Zügen.
- [ ] **Greedy**: Hex-/Torus-Distanz zum Futter minimieren.
- [ ] **Pfadplaner**: A* zum Futter + Survival-Check (Schwanz nach
      simuliertem Pfad erreichbar?), sonst Tail-Chasing.
- [ ] Benchmark-Harness in `snake-core` (headless, N Partien pro Strategie,
      ⌀-Score/⌀-Überlebenszeit) — als Test/Beispiel-Binary.
- [ ] Autopilot-Läufe werden vom Highscore ausgenommen (oder eigene Tabelle).

**Done wenn:** Pfadplaner spielt sichtbar gute Partien in beiden Randmodi;
Benchmark zeigt Chaos < Greedy < Pfadplaner.

## Phase 5 — Weitere Strategien

- [ ] **Raumgreifer**: Flood-Fill-Bewertung pro Zug, Futter als Tiebreaker.
- [ ] **Hamilton-Fahrer**: Serpentinen-Hamilton-Zyklus konstruieren
      (Generator + Test, dass jeder Zyklus gültig ist); Test stellt sicher,
      dass **alle drei Presets kompatibel** sind. Shortcut-Logik entlang der
      Zyklusordnung; bei frei gewählten, inkompatiblen Maßen wird die
      Strategie ausgegraut (Tooltip erklärt warum).
- [ ] **Monte-Carlo-Lookahead**: Rollouts mit Tick-Budget, Parameter
      (N, Horizont) als Konstanten mit sinnvollen Defaults.
- [ ] **Debug-Overlay**: A*-Pfad, Flood-Fill-Heatmap, MC-Bewertung je
      Richtung einblendbar.

**Done wenn:** Fünf+ Strategien wählbar, Overlay zeigt nachvollziehbar, was
die KI „denkt".

## Phase 6 — Neuronales Netz

- [ ] Sensor-Featurevektor in `snake-core` (6 Richtungsstrahlen: Distanz zu
      Hindernis/Körper/Futter, + Richtung/Länge).
- [ ] Mini-MLP (Forward-Pass pur in Rust, keine externen ML-Deps);
      Gewichts-(De)Serialisierung in einem simplen, dokumentierten Format —
      dasselbe Format nutzt später auch der Python-Export (Phase 9).
- [ ] `snake-train`: Evolutionsstrategie/GA, Fitness = Score +
      Überlebenszeit, parallelisiert (rayon), Checkpoints speichern.
- [ ] Trainieren, bestes Netz + 2–3 Zwischen-Generationen als Assets
      einbetten; Strategie „Neural Net (Gen X)" im Dropdown.
- [ ] NN in den Benchmark aufnehmen.

**Done wenn:** Das NN schlägt Greedy im Benchmark deutlich und ist im Browser
wählbar.

## Phase 7 — Gimmicks, Welle 1

- [ ] Spezialfutter: goldener Apfel (Timeout), fauler Apfel, Tempo-Frucht —
      im Core inkl. Tests, dann UI; per Menü abschaltbar („Klassisch-Modus",
      bleibt Default für Highscores).
- [ ] Replay: Seed + Inputliste aufzeichnen, Replay-Player im UI,
      Ghost-Anzeige des besten eigenen Laufs.
- [ ] **Theme-/Skin-System**: Theme-Abstraktion als reine Daten + Zeichenstil
      (Farben, Kopf-/Segment-Form, Futter-Darstellung) — kein Zugriff auf
      die Spiellogik, prozedural gezeichnet (keine Sprite-Assets). Auswahl
      im Menü, Wahl wird persistiert.
- [ ] Skins umsetzen: **Klassisch** (cleane Geometrie), **Honigwabe**
      (Wabenoptik, Raupe/Bienen, Honigtropfen als Futter), **Retro-LCD**
      (monochromes Nokia-Grün), **Neon** (dunkler Grund, Glow via
      halbtransparente Outlines), **Naturalistisch** (Kopf mit Augen, die
      Richtung Futter blicken; zum Schwanz schmaler werdende Segmente,
      Apfel als Futter).
- [ ] **Farbenblind-sichere Palette** als Pflicht-Theme: Schlange, Futter
      und Spezialfutter unterscheiden sich auch über Form, nicht nur Farbe.
- [ ] Fress-/Game-Over-Effekte (theme-übergreifend).
- [ ] Touch-Steuerung (virtuelles Hex-Pad) für Mobile.
- [ ] Statistik-Panel (Spiele, ⌀-Länge, Bestwerte).

**Done wenn:** Klassisch- und Gimmick-Modus spielbar, Replays abspielbar.

## Phase 8 — Optionaler Server

- [ ] `snake-server`: axum, SQLite; Endpoints `GET/POST /highscores/{mode}`.
      Globale Leaderboards nur für die drei Presets; Identität ist ein frei
      wählbarer Name, das Submit-Schema enthält aber von Anfang an ein
      **optionales Signaturfeld** (Keypair-Nachrüstung später ohne
      Migration).
- [ ] Verifikation: Client sendet Seed + Inputliste, Server re-simuliert mit
      `snake-core` (gleiche Crate ⇒ gleiches Verhalten) und akzeptiert nur
      konsistente Läufe.
- [ ] **Härtung der öffentlichen Endpoints**:
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
- [ ] Client: `ehttp`-Anbindung mit Timeout; nicht erreichbar ⇒ stilles
      Fallback auf lokale Tabelle, ausstehende Läufe werden lokal gemerkt
      und später nachgereicht. UI zeigt lokale und globale Tabelle.
- [ ] Daily Challenge: Tagesseed vom Server, lokaler datumsbasierter
      Fallback; eigenes Leaderboard.
- [ ] **All-in-one-Dockerfile**: Multi-Stage-Build (Stage 1: trunk-Release-
      Build des WASM-Frontends; Stage 2: cargo-Build des Servers; Runtime-
      Stage: schlankes Image). Der axum-Server liefert neben der API auch
      die statischen Dateien aus (`tower-http` `ServeDir`) — damit ist das
      komplette Spiel inkl. Highscore-Server als ein Container hostbar.
      SQLite-Datei auf einem Volume (`/data`), Pfad per Env-Var.
      Container gehärtet: läuft als Non-Root-User, Root-Filesystem
      read-only (beschreibbar nur `/data`), keine zusätzlichen
      Capabilities; Beispiel-`docker-compose.yml` mit diesen Optionen
      beilegen.
- [ ] Client-Konfiguration für beide Hosting-Varianten: API-Aufrufe gehen
      per Default an **relative URLs** (same-origin — deckt die Docker-
      Variante ohne CORS ab); für den GitHub-Pages-Build wird die
      Server-URL zur Buildzeit/per Konfiguration gesetzt. GitHub Pages
      bleibt das primäre Hosting.
- [ ] Deployment-Notizen (`docs/deployment.md`) mit drei Varianten:
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

Ziel: ein Strategie-Dropdown mit GA/ES, NEAT, Behavior Cloning, DQN und PPO,
die im Benchmark gegeneinander antreten (vgl. Konzept §3.8).

- [ ] **NEAT** in `snake-train` (bleibt im Rust-Track, kein Gradient nötig);
      optional **CMA-ES** als alternativer Optimierer für das bestehende MLP.
- [ ] **PyO3-Bindings**: `snake-core` via maturin als Python-Modul
      (Gym-artiges Env-Interface: `reset`/`step`/Observation), Python-Setup
      unter `python/` (uv/venv, dokumentiert).
- [ ] **Behavior Cloning**: Datensatz-Generator (Pfadplaner spielt headless,
      loggt Zustand→Zug), Supervised Training in PyTorch, Export ins
      Rust-Gewichtsformat.
- [ ] **DQN und PPO** mit `stable-baselines3` gegen das PyO3-Env;
      Reward-Shaping (Futter, Überleben, Freiraum) dokumentieren. Export der
      Policy-Netze ins Rust-Gewichtsformat (Inferenz bleibt pur Rust/WASM).
- [ ] Optional: **CNN-Input** (Brett als Gitter-Tensor) als alternative
      Observation für DQN/PPO, Vergleich gegen Sensorstrahlen.
- [ ] Alle Verfahren als Dropdown-Einträge + Aufnahme in den
      Benchmark-Harness (Vergleichstabelle ⌀-Score/⌀-Überlebenszeit).
- [ ] Optional (anspruchsvollste Stufe): **AlphaZero-light** — Policy/Value-
      Netz ersetzt die Zufalls-Rollouts des Monte-Carlo-Lookahead, Training
      per Self-Play.

**Done wenn:** Mindestens GA/ES, Behavior Cloning, DQN und PPO sind im
Browser wählbar und der Benchmark zeigt eine Vergleichstabelle.

## Spätere Ausbaustufen (bewusst nicht geplant)

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
