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

## Phase 3 — Lokale Highscores

- **Die request_focus-Falle**: Ein Dialog-TextEdit, das sich bei „nichts
  fokussiert" jeden Frame selbst den Fokus zurückholt, macht das kanonische
  egui-Muster `lost_focus() && key_pressed(Enter)` unmöglich — beim Enter
  gibt das Feld den Fokus ab, der Auto-Refocus holt ihn im selben Frame
  zurück, und `lost_focus()` (definiert als „hatte Fokus letzten Frame,
  hat ihn jetzt nicht") bleibt für immer false. Lösung: Fokus nur einmalig
  beim Öffnen des Dialogs anfordern. Gefunden über eine
  Playwright-Screenshot-Sequenz.
- **Deterministische E2E-Tests dank `?inputs=`**: Seed *und* Inputliste als
  URL-Parameter (ein Zeichen = ein Tick) machen Browser-Tests von UI-Flows
  trivial: Ein 30-Zeichen-Skript (von einem Greedy-Generator in `snake-core`
  erzeugt) spielt im Browser zuverlässig zu „Score 2, Game Over" — kein
  fragiles zeitbasiertes Steuern per Tastatur-Events nötig.
- **Datum ohne Datums-Crate**: Howard Hinnants `civil_from_days` sind ~15
  Zeilen Integer-Arithmetik und ersetzen chrono komplett, wenn man nur
  „YYYY-MM-DD aus Unix-Zeit" braucht — funktioniert identisch in nativ
  (SystemTime) und WASM (js_sys::Date::now).
- **eframe speichert im Browser nur alle 30 s** (auto_save_interval-Default)
  — wer den Tab vorher schließt, verliert Highscores. Auf 5 s verkürzt;
  der Reload-Test im Browser muss entsprechend warten.

## Phase 4 — Autopilot & erste Strategien

- **Zeitbewusstes A\***: Statt alle Schlangenzellen als Hindernis zu
  blocken, bekommt jede Körperzelle eine „Frei-ab"-Zeit (Schwanz: Tick 1,
  Kopf: Tick Länge). A* darf durch Zellen planen, die der Schwanz bis zur
  Ankunft geräumt hat — deutlich stärkere Pfade als beim naiven Blocken,
  ohne Mehraufwand (eine Vec-Indizierung statt HashSet-Lookup).
- **Der Survival-Check ist eine echte Zukunftssimulation**: Weil das Spiel
  deterministisch ist (Seed-RNG im GameState), ist `state.clone()` +
  geplante Züge ticken nicht bloß eine Näherung, sondern exakt die
  Zukunft — inklusive des Futter-Respawns. Der Determinismus, ursprünglich
  für Replays/Server-Verifikation gebaut, fällt der KI gratis zu.
- **Benchmark-Zahlen (16×12, 50 Partien, max 10k Ticks)**:
  | Strategie | Walls ⌀ | Torus ⌀ |
  |---|---|---|
  | Chaos-Walker | 2,36 | 7,56 |
  | Greedy | 23,40 | 32,96 |
  | Pfadplaner | 144,76 | **188,02** |
  Auf dem Torus ist 189 der Maximalscore (192 Zellen − Startlänge 3) — der
  Pfadplaner füllt dort das Brett fast in jeder Partie *komplett* und
  gewinnt. Ohne Wände gibt es schlicht weniger Sackgassen, und schon der
  simple Tail-Chase-Fallback reicht zum Perfect Game.
- **Auch der Zufalls-Walker profitiert vom Torus** (2,4 → 7,6): Wände sind
  für alle Strategien der Haupttodesgrund.
- **First-Step-Regel im A\***: Der erste Pfadschritt darf nie die
  Gegenrichtung sein — das Spiel würde den Zug ignorieren und geradeaus
  weiterfahren. Leicht zu übersehen, weil es nur in Eck-Situationen
  auffällt.

## Phase 4b — Bedienkomfort & Seed-Darstellung

- **Eindeutigkeit von Base64-Seeds gratis durch ein freies Nibble**: Ein
  32-Bit-Seed in 6 Base64-Zeichen belegt nur 32 von 36 Bits — die obersten
  4 Bits der ersten 6-Bit-Gruppe sind immer 0, das erste Zeichen damit
  immer `A`–`P`. Ein kodierter Seed kann also nie „nur aus Ziffern"
  bestehen, eine Dezimalzahl immer — die Regel „nur Ziffern ⇒ dezimal,
  sonst Base64" ist beweisbar kollisionsfrei, ganz ohne Präfix oder
  Marker. (Per Unit-Test festgenagelt.)
- **Auto-Pause bei offenem Dropdown** ist in egui ein Einzeiler: Die
  `ComboBox`-`InnerResponse` hat `inner == Some(…)` genau dann, wenn das
  Popup offen ist — daraus pro Frame ein `ui_paused`-Flag ableiten und im
  Tick-Scheduler zusätzlich `next_tick` verwerfen (sonst spult das Spiel
  die Pause nach dem Schließen nach). Verifiziert per zwei
  Playwright-Screenshots im Abstand von 2 s: identische Schlangenposition.
- **Highscore bei Tempowechsel: das langsamste Tempo zählt**. Erst war
  geplant, solche Läufe komplett auszuschließen; die fairere Regel
  (Wertung in der Tabelle des langsamsten verwendeten Tempos) ist genauso
  einfach zu implementieren: ein `min()` über ein `Ord`-Enum pro Wechsel.

## Phase 5 — Weitere Strategien & Debug-Overlay

- **Hamilton auf dem Hexgitter ist ein Geschenk des odd-q-Layouts**: In
  Offset-Koordinaten existiert ein „horizontaler" Lauf bei konstanter Zeile
  (abwechselnd SE-/NE-Schritte), und N/S bleiben in der Spalte — damit
  funktioniert die klassische Quadratgitter-Serpentine (Spalte 0 als
  Rückweg, Rest boustrophedon) eins zu eins auf Hexes. Einzige Bedingung:
  gerade Zeilenzahl. Der Validierungstest (jede Zelle genau einmal, alle
  Übergänge adjazent, Zyklus geschlossen) ist Pflicht — Paritätsfehler
  sieht man sonst erst beim Spielen.
- **Der Hamilton-Test schlug fehl, weil die Strategie *gewann***: Der Test
  prüfte `status == Running` nach 8000 Ticks, aber der Fahrer hatte das
  Brett komplett gefüllt (`Won`, Score 189 = Maximum auf 16×12). Der
  Shortcut-Check über Vacate-Zeiten entlang der Zyklusordnung reicht also
  für Perfect Games in beiden Randmodi.
- **Eine `StrategyDebug`-Struktur statt Spezial-APIs**: Pfad, Heatmap und
  Richtungs-Scores als gemeinsames Format, das jede Strategie nach Bedarf
  füllt — das Overlay kennt keine Strategien, nur diese drei Felder.
  Beim Hamilton-Fahrer wird daraus gratis eine Zyklus-Visualisierung.
- **`from_rgba_unmultiplied` ist nicht `const`** — für Farbkonstanten in
  egui muss man premultiplied-Werte vorrechnen, sonst kompiliert es nicht
  (oder man nimmt versehentlich premultiplied mit unmultiplied-Werten und
  bekommt grelles Türkis, so gefunden per Screenshot).

## Phase 6 — Skins, Mobile & Statistik

- **Theme = statische Daten, Renderer = ein Match**: Sechs Skins sind eine
  Palette plus drei Stil-Enums (`SnakeStyle`, `FoodStyle`, `HeadMarker`);
  der Renderer kennt keine Themes, nur diese Felder. Neue Skins sind
  dadurch reine Datendefinitionen (`static NEON: Theme = …`).
- **„Schlängeln sichtbar" über Gittervektoren**: Die Pixel-Verschiebung
  eines Hex-Schritts ist paritätsunabhängig (reiner Gittervektor, Länge
  √3·s für alle sechs Richtungen) — damit ist das Körperband trivial:
  Kreise an den Zentren + dicke Linien je Segment. Am Torus-Wrap werden
  statt einer Linie quer übers Brett zwei Stummel gezeichnet, die durch
  die gemeinsame Kante austreten (halber Gittervektor) — gleiche Idee wie
  der Linienabbruch im Debug-Overlay.
- **Glow ohne Shader**: Drei Pässe desselben Bandes mit wachsender Breite
  und sinkendem Alpha (`gamma_multiply`) ergeben einen brauchbaren
  Neon-Effekt in egui — kein Postprocessing nötig.
- **Augen, die zum Futter schauen**, sind im Screen-Space ein
  Zweizeiler (normalisierter Blickvektor + Senkrechte), wirken aber
  überproportional lebendig — der billigste „Charme-pro-Zeile"-Gewinn des
  Projekts.
- **Touch-Pad aus Gittervektoren**: Die sechs Richtungsknöpfe liegen auf
  einem Ring — die Positionen sind exakt die sechs Hex-Gittervektoren,
  wiederverwendet aus dem Band-Rendering. Eintritts-Bedingung
  `any_touches()` hält das Pad von Desktop-Bildschirmen fern.
- **Teststrategie-Kurskorrektur**: Die Ad-hoc-Playwright-Skripte mit
  Pixel-Koordinaten waren als Einmal-Verifikation okay, als Tests
  unbrauchbar (drei Koordinaten-Rekalibrierungen in einer Session).
  Jetzt dreistufig: Logik headless in snake-core, UI-Zustände als
  egui_kittest-Snapshots (deterministisch, da nativer Renderer + Seed +
  simulierte Uhr), und *ein* tastaturgetriebener Browser-Smoke-Test mit
  localStorage-Assertions statt Screenshots.
- **egui_kittest-Aha**: Die simulierte Uhr springt pro `step()` direkt zum
  nächsten `request_repaint_after`-Termin — bei festem Game-Tick heißt das
  exakt **1 step = 1 tick**. Erst verwirrend (Spiel „zu schnell"), dann
  das perfekte Werkzeug: Tick-genaues Vorspulen ohne Zeitrechnerei.
- **HUD frisst das Brett auf Mobil**: Auf schmalem Viewport (390 px) war
  das Spielfeld an den Seiten abgeschnitten. Ursache war nicht das Brett,
  sondern die *eine* horizontale HUD-Zeile: breiter als der Bildschirm,
  blähte sie `ui.available_size()` auf, womit der Brett-Painter größer als
  der Viewport wurde und das (zentrierte) Brett halb hinausragte. Fix:
  Painter auf `ctx().content_rect()` kappen und ins sichtbare Rechteck
  zeichnen (`response.rect.intersect(screen)`) — so bleiben Brett *und*
  Pause/Game-Over-Overlay zentriert und vollständig sichtbar.
- **Geometrischer e2e-Test statt Pixel-Diff**: Für „Brett komplett
  sichtbar" reicht kein localStorage-Check und ein Snapshot wäre zu fragil.
  Lösung: Screenshot mit Node-`zlib` selbst zu RGBA dekodieren und nur eine
  *geometrische* Eigenschaft prüfen — die horizontale Ausdehnung der
  Brett-Inhalte (mittleres Helligkeitsband: Wand/Schlange/Futter, nicht die
  helle egui-Rahmen- und nicht die dunkle Brett-Hintergrundfläche) muss
  zentriert und beidseitig vom Rand abgesetzt sein. Robust gegen Font-/
  Rendering-Unterschiede, fängt die Regression aber zuverlässig (Span 4 px
  statt ~370 px im Fehlerfall).

## Phase 7 — Neuronales Netz (GA/ES)

- **Der Benchmark als Bug-Detektor**: Beim Aufnehmen des NN fiel auf,
  dass der Raumgreifer in 20 Partien exakt 0,00 Punkte holte — er
  verhungerte vor vollem Brett. Zwei gestapelte Ursachen: (1) Fressen
  kostet immer genau eine Zelle Freiraum (die Schlange wächst), ein
  strikter Flood-Fill-Vergleich meidet Futter also *systematisch*;
  (2) der Futter-Tiebreaker maß die Distanz zum bereits respawnten
  Futter — der Fress-Zug verlor damit jeden Tiebreak. Lehre: ⌀-Score
  über viele Partien entlarvt Verhaltensfehler, die im Zuschauen
  („überlebt doch ewig!") unsichtbar bleiben.
- **Rotationsinvariante Features**: Die sechs Sensorstrahlen sind relativ
  zur Blickrichtung indiziert (0 = geradeaus), die Netz-Outputs ebenso —
  das Netz muss „links ist frei" nur einmal lernen statt sechsmal. Auf
  dem Torus wrappt der Strahl und das einzige Hindernis, das er finden
  kann, ist der eigene Körper von hinten (eigener Test).
- **f32-Roundtrip gratis**: Rusts `{:?}`-Formatierung druckt die kürzeste
  exakt round-trippende Dezimaldarstellung — das Textformat für Gewichte
  braucht deshalb kein Binärformat, bleibt diffbar und ist trotzdem
  bitgenau.
- **Schon der Smoke-Run schlägt Greedy**: 120 Generationen, Population
  48, ~1 Minute auf dem Laptop ⇒ ⌀ 37,4 (Walls 16×12) vs. Greedy 21,0.
  Benchmark aller acht Strategien (20 Partien, max 5000 Ticks, Walls):
  Chaos 2,1 < Greedy 21,0 < NN-Smoke 37,4 < Monte-Carlo 62,4 <
  Pfadplaner 134,8 < Hamilton 180,4 (Torus sogar 189,0 = jede Partie
  perfekt). Der echte Trainingslauf läuft später auf stärkerer Hardware
  (`docs/training/neural-net-ga.md`).
- **Box–Muller statt rand_distr**: Für Gauß-Mutation reichen zwei Zeilen
  Box–Muller — eine Dependency weniger im Trainer.
