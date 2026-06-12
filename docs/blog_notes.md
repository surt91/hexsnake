# Blog-Notizen — HexSnake

Rohmaterial für einen späteren Blog-Post über die Entwicklung von HexSnake.
Während der Implementierung wird hier alles Bemerkenswerte festgehalten:
Aha-Momente, Stolpersteine, Designentscheidungen, schöne Tests, API-Fallen.
Format: ein Bullet pro Erkenntnis, gruppiert nach Phase, mit genug Kontext,
dass man es Monate später noch versteht.

## Phase 0 — Setup

- **eframe 0.34 hat die App-API umgebaut**: `App::update(ctx, frame)` ist
  deprecated, Pflichtmethode ist jetzt `App::ui(&mut self, ui, frame)` — man
  bekommt direkt ein randloses Root-`Ui` statt eines `Context`. Entsprechend
  ist `CentralPanel::show` deprecated zugunsten von
  `egui::Frame::central_panel(ui.style()).show(ui, …)`. Praktisch alle
  Online-Templates/Tutorials zeigen noch die alte API — der Compiler-Fehler
  `missing ui in implementation` ist der Wegweiser.
- Das offizielle eframe-Web-Template (Canvas per ID suchen,
  `WebRunner::start` in `wasm_bindgen_futures::spawn_local`) funktioniert
  unverändert mit trunk 0.21; `<link data-trunk rel="rust" />` +
  `<base data-trunk-public-url />` reichen als gesamte Build-Konfiguration.

## Phase 1 — Hexgitter & Spiellogik

- **Flat-Top-Hexagons passen perfekt auf QWEASD**: die sechs Nachbarn sind
  genau N/NE/SE/S/SW/NW — es gibt kein „Ost/West", was auf der Tastatur
  ohnehin fehlen würde. Die Richtungs-Deltas in axialen Koordinaten sind
  trivial (je eine Komponente ±1 oder 0).
- **Axial ↔ Offset (odd-q) ohne Floor-Akrobatik**: In der Konvertierung
  `row = r + (q - (q & 1)) / 2` ist `q - (q & 1)` immer gerade — die
  truncating Integer-Division von Rust ist hier also exakt, auch für
  negative `q` (Rusts `&` auf negativen Zahlen arbeitet auf dem
  Zweierkomplement: `-3 & 1 == 1`). Kein `div_euclid`/Floor nötig; ein
  Roundtrip-Test über negative Bereiche sichert das ab.
- **Torus-Distanz = Minimum über 9 Wrap-Varianten**: Für periodische Ränder
  wird der Zielpunkt in Offset-Koordinaten um ±Breite/±Höhe verschoben und
  das Minimum der Hex-Distanzen genommen. Wichtigster Test dazu ist eine
  Eigenschaft, kein Beispiel: *jeder* per Wrap erreichte Nachbar muss
  Torus-Distanz 1 haben. Der Test deckt subtile Paritätsfehler auf, die
  Beispiel-Tests leicht verfehlen (Hex-Spalten verschieben sich beim
  horizontalen Wrap je nach Paritätskombination).
- **Determinismus über `PartialEq` auf dem kompletten GameState**: Der
  RNG (`Pcg32` aus `rand_pcg`) implementiert selbst `PartialEq` — dadurch
  kann der Determinismus-Test einfach zwei komplette `GameState`-Instanzen
  (inklusive RNG-Zustand!) Tick für Tick auf Gleichheit vergleichen, statt
  einzelne Felder zu samplen.
- **Futter-Respawn ohne HashMap-Falle**: freie Zellen werden in fester
  Row-Major-Reihenfolge gesammelt und per RNG-Index gewählt. Ein
  `HashSet`-basierter Ansatz wäre naheliegend, aber dessen
  Iterationsreihenfolge ist nicht deterministisch — das würde die spätere
  Server-Verifikation der Replays still brechen.
- **Schwanz-Regel**: Der Zug auf die Zelle des eigenen Schwanzendes ist
  erlaubt, *außer* die Schlange frisst gerade (dann rückt der Schwanz nicht
  nach). Klassische Snake-Subtilität, die man leicht falsch implementiert;
  eigener Test dafür.
- **Tests als Mini-KI**: Der Test-Helper `step_toward` (greedy Richtung
  Futter, tödliche Züge ausgeschlossen) ist quasi ein Vorgriff auf die
  Greedy-Strategie aus Phase 4 — damit spielt der Unit-Test komplette
  Partien headless von Start bis Game Over und übt Fressen, Wachsen,
  Respawn und Kollision in einem Durchlauf.

## Phase 2 — Spielbares Frontend

- **Das 300×150-Pixel-Spiel**: eframe ≥0.34 übernimmt die Canvas-Größe aus
  dem CSS — setzt man kein `width/height: 100%`, rendert das komplette Spiel
  in die HTML-Default-Canvas-Größe von 300×150 px (überlappender Text,
  Mini-UI). Ältere eframe-Templates ließen eframe das Canvas selbst
  aufziehen; beim Upgrade ist das eine stille Falle. Gefunden per
  Playwright-Screenshot, der das Symptom sofort sichtbar machte.
- **UI-Verifikation per Headless-Browser**: trunk-Dev-Server + Playwright-
  Skript (Canvas anklicken — sonst kommen Tastatur-Events nicht an! — dann
  QWEASD senden, Screenshot). Der Beweis-Screenshot für den periodischen
  Modus zeigt die Schlange mitten im Wrap: Kopf am unteren, Körper noch am
  oberen Rand — besser kann man Torus-Topologie kaum illustrieren.
- **Tick-Scheduling ohne Backlog**: Der nächste Tick wird relativ zur
  Fälligkeit geplant (kein Drift), aber mit `max(now)` geklemmt — sonst
  „spult" das Spiel nach einem versteckten Browser-Tab alle verpassten
  Ticks im Schnelldurchlauf nach. Beim Pausieren wird der Timer verworfen.
- **Randbedingung als Optik**: Wände = massiver Rahmen, Torus = gestrichelte
  Linie („durchlässig") — die Metapher trägt erstaunlich gut, ganz ohne
  Erklärtext.
