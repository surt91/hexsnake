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
  (`docs/training/mlp-ga/guide.md`).
- **Box–Muller statt rand_distr**: Für Gauß-Mutation reichen zwei Zeilen
  Box–Muller — eine Dependency weniger im Trainer.

## Phase 8 — Optionaler Server

- **Verifikation als reines Re-Play, ohne Score im Submit**: Das `Replay`
  trägt nur Brett, Seed und eine *sparse* `(tick, direction)`-Liste — keinen
  Score. Der Server re-simuliert mit derselben `snake-core`-Crate und nimmt
  den selbst errechneten Score; ein Client kann seine Punktzahl gar nicht
  fälschen, weil sie nie übertragen wird. Die `Recorder`/`Replay`-Trennung
  hält die Aufzeichnung (im UI) und die Wiedergabe (im Server) entkoppelt.
- **Determinismus zahlt sich aus**: Dass `snake-core` seit Phase 1 strikt
  deterministisch ist (seedbarer `Pcg32`, Reservoir-Sampling statt
  HashMap-Reihenfolge), macht die Server-Verifikation zu einem Einzeiler —
  gleiche Inputs ⇒ gleicher Verlauf, sonst 4xx. Der `verify()`-Loop
  verbraucht Inputs „bei Tick-Gleichheit" und lehnt alles ab, was die Partie
  nie erreicht (out-of-order, nach Spielende) oder ein hartes Tick-Budget
  sprengt (Torus-Geradeauslauf endet nie).
- **Speed ist nicht verifizierbar — und muss es nicht sein**: Die
  Geschwindigkeit beeinflusst nur die Echtzeit-Taktung, nicht die
  Simulation, taucht also nicht im Replay auf. Sie selektiert nur eine von 9
  Tabellen; man kann damit keinen Score aufblähen, nur die Tabelle wählen.
- **Hand-gerolltes Rate-Limit statt `tower-governor`**: Ein
  Fixed-Window-`HashMap<IpAddr, (start, count)>` hinter einem Mutex ist für
  eine einzelne Hobby-Instanz genug — und spart eine fragile Abhängigkeit
  samt axum-Versions-Kompatibilität. `X-Forwarded-For` wird nur gelesen,
  wenn explizit konfiguriert (`TRUST_FORWARDED_FOR`), sonst die Peer-IP.
- **`Option<ConnectInfo>` geht in axum 0.8 nicht** (kein
  `OptionalFromRequestParts` dafür) — ein eigener `ClientIp`-Extractor
  kapselt die XFF-/Peer-Logik sauber und ist in `oneshot`-Tests
  (ohne ConnectInfo) per `0.0.0.0`-Fallback testbar.
- **Offline bleibt offline**: Der Client schickt per `ehttp` (nativ + WASM)
  rein best-effort; Ergebnisse kommen über einen Channel, den das UI je
  Frame leert. Server weg ⇒ stilles Zurückfallen auf lokale Tabellen,
  offline gespielte Läufe landen in einer persistierten Pending-Queue und
  werden gedrosselt nachgereicht. Die Server-URL kommt aus
  `option_env!("SNAKE_SERVER_URL")` (Pages-Build), sonst same-origin im
  Browser (Docker) bzw. Laufzeit-Env nativ.
- **Daily offline ehrlich gemacht**: Der Tagesseed kommt vom Server (mit
  Secret), offline gibt es einen datumsbasierten Fallback-Seed *ohne*
  Secret. Solche Läufe ranken bewusst nur lokal — ihr Seed passt nicht zum
  Server und würde bei der Re-Simulation zu Recht abgelehnt.

## Phase 9 — ML-Ausbaustufe (NEAT, DQN/PPO, AlphaZero-light)

- **Inferenz vom Lernen entkoppelt**: Jedes Verfahren landet am Ende als
  pures Rust-Netz (MLP- bzw. NEAT-Genom), das nativ und in WASM läuft —
  trainiert wird in Rust (GA/NEAT/ES) oder Python (DQN/PPO), aber zur Laufzeit
  kein PyTorch. Die Dropdown-Einträge sind „nur" austauschbare Gewichts-
  Assets. Genau deshalb passt DQN/PPO ins selbe `.mlp`-Format: SB3 mit
  `activation_fn=Tanh` und `net_arch=[32,24]` ergibt exakt `[20,32,24,6]`,
  `argmax` über Q-Werte/Logits = `argmax` über die MLP-Ausgaben.
- **NEAT-Inferenz = ein Topo-Sort**: Das Genom (Knoten + innovationsnummerierte
  Kanten) wird einmal zu einem `Net` mit topologischer Auswertungsreihenfolge
  (Kahn) kompiliert; `compile()` assert't damit zugleich, dass die Mutationen
  feed-forward geblieben sind. Der Zyklus-Check bei „Kante hinzufügen" ist eine
  Erreichbarkeitssuche von `to` nach `from`.
- **PyO3-Env testbar ohne Python**: Die gesamte RL-Logik (reset/step/Reward/
  Observation) liegt in einem reinen Rust-`Env`; der `#[pymodule]` ist ein
  dünner Wrapper hinter einem optionalen `python`-Feature. So testet
  `cargo test --workspace` das Env ohne Python-Toolchain, und maturin baut die
  Extension mit `--features python`.
- **Roundtrip-Test ohne torch**: Die klassische Transponierungs-/Layout-Falle
  beim Gewichtsexport wird per `verify_roundtrip.py` gegen die *echte*
  Rust-Inferenz geprüft — eine `mlp_forward`-Binding lässt numpy-Export und
  `snake-core`-Forward auf Zufallsinputs vergleichen (max. Fehler 5e-6),
  ganz ohne stable-baselines3.
- **AlphaZero-light: Such-Budget ist Teil des Modells**: Mit GA evolviert
  (gradientenfrei) und mit `--sims 16` trainiert, spielte das Netz mit
  `--sims 48` plötzlich *schlechter* — es vertraut dem Value-Kopf zu tief und
  kreist sicher, statt zu fressen. Lehre: Der Value ist nur für die
  Trainings-Tiefe kalibriert; `embedded()` muss dasselbe Budget nutzen.
- **Ein `evolve` für drei Trainer**: GA-MLP und AlphaZero-light teilen sich
  denselben ES-Kern (Truncation + Gauß-Mutation, rayon-parallel) — der
  Unterschied ist nur die `build: &[f32] -> Box<dyn Strategy>`-Closure und die
  Netz-Dimensionen.

## GA-Training Runs — Serie (Runs 001–006)

- **Budget dominiert mehr als Architektur**: Die Hypothese aus Run 001 „mehr
  Features oder größere Netze helfen" wurde erst in Run 005 wirklich bestätigt
  — weil Run 004 bei größerem Netz *weniger* evals/param hatte als Run 003.
  Sobald das Budget pro Parameter gleich bleibt, gewinnt das größere Netz.
  Lernkurve: 003 (618 P, 828 evals/P) → 004 (1614 P, 317 evals/P) → 005
  (1614 P, 2380 evals/P) = 91.4/125.1, besser als alle vorherigen Runs.

- **Seed-Sensitivität kann Budget schlagen**: Run 006 lief 4× länger als
  Run 005, aber Seed 4 konvergierte in ein schlechteres lokales Optimum.
  Schon bei gen 5000 lag Run 006 (fitness 9588) weit hinter Run 005 (12434)
  — auf denselben Eval-Boards, da die Seeds generationsindexiert sind.
  Mit Truncation-ES gibt es keinen Mechanismus, dieses Tal zu verlassen:
  σ = 0.06 ist zu klein für den 1614-dim-Parameterraum.

- **Budget-Hypothese praktisch**: Mehrere kurze Runs mit verschiedenen Seeds
  sind effizienter als ein langer Lauf mit fixem Seed. Wer länger trainieren
  will, sollte parallel mehrere Seeds starten und am Ende das beste Netz
  wählen (Selection-over-Restarts). CMA-ES wäre der sauberere Ansatz:
  adaptive Schrittweite + Kovarianz-Anpassung umgeht die Plateau-Falle.

- **Topologie-Blindheit ist ein echter Bug**: Run 001 (Walls-only) erzielte
  auf Periodic nur Ø 6 — das Netz hatte nie gelernt, dass Wände fehlen.
  `--mixed` (50/50 Walls/Periodic) löste das Problem vollständig. Preis:
  Walls-Score fiel kurzzeitig, erholte sich aber mit mehr Budget.

- **Continuous food-approach > binär**: Der Wechsel von binärem
  „approaches_food" zu einem signierten Wert (Magnitude ∝ 1/food_dist) war
  konzeptuell richtig, aber in Gen 100 war der Score kurzfristig schlechter.
  Das Netz muss erst lernen, den neuen Signal-Bereich zu nutzen — danach
  übertrifft es klar das binäre Signal. Geduld zahlt sich aus.

- **Fitness-Werte sind nicht cross-Run vergleichbar**: Run 006 gen-0 best=2223,
  Run 005 gen-0 best=983 — trotzdem ist Run 006 schwächer. Seed 4 zieht
  zufällig eine stärkere Initialpopulation, aber das sagt nichts über das
  Endresultat. Nur gleiche Generation = gleiche Eval-Boards → direkt
  vergleichbar.

## Phase 9 — AlphaZero-light auf Gradienten (Nachtrag)

- **Eine MCTS, kein Sync-Problem**: Statt die Suche in Python für das Training
  und in Rust für die Inferenz doppelt zu pflegen (Divergenz-Risiko), läuft
  das komplette Self-Play in Rust (`az_selfplay`) — exakt dieselbe Suche wie
  die Inferenz. Python bekommt nur (Features, MCTS-Verteilung, Return) und
  macht den Gradientenschritt. Der GIL wird während der reinen Rust-Suche
  freigegeben → ein ThreadPool fächert Self-Play über alle Cores (user 4m42
  bei real 37s).
- **Cold-Start-Falle**: Erster Versuch lernte, *ewig im Kreis zu fahren*
  (Score 0, Ticks am Limit). Ursache: Bei sparsamen Fress-Reward und zufällig
  initialisiertem Value findet die 24-Sim-Suche nie Futter; die
  Besuchsverteilung bleibt diffus, die Policy lernt „irgendein sicherer Zug".
  Dichteres Reward im *Trainings-Target* allein half nicht — die **Suche
  selbst** brauchte das Signal.
- **Reward-aware Search als Fix**: Die MCTS-Kantenwerte enthalten jetzt das
  dichte Schritt-Reward (Futter-Annäherung + Fress-Bonus), nicht nur den
  Netz-Value am Blatt. Damit steuert die Suche schon mit untrainiertem Netz
  Futter an, die Policy-Targets werden informativ — und das Gradienten-Netz
  frisst plötzlich (Walls 28 / Torus 54, über der GA-Variante 17/52). Lehre:
  Bei AlphaZero für eine Sparse-Reward-Aufgabe muss das dichte Signal *in der
  Suche* stecken, nicht nur im Lernziel.
- **7-Output-Roundtrip**: Der Export-Self-Check (Torch-Raw == Rust
  `mlp_forward`, Fehler ~1e-7) fängt Layout-Fehler des Policy+Value-Kopfs ab,
  bevor ein Trainingslauf verschwendet wird.
- **max-ticks als Trainingsregler**: `--max-ticks 1500` erzeugt einen natürlichen
  Sweet Spot: Die self-play Spiele werden auf 1500 Ticks begrenzt, was die
  Schlange zum effizienten Fressen zwingt. Das optimale Fenster liegt bei
  game_len ~700–900 (≈50–60 % des Limits). Darüber lernt die Schlange zu
  kreisen statt zu fressen — erkennbar an avg_ticks >1000 im Benchmark.
- **best.mlp ≠ bester Checkpoint**: Die Trainingskurve speichert best.mlp wenn
  game_len ein neues Maximum erreicht. Aber maximale game_len im Self-Play
  korreliert nicht perfekt mit dem Benchmark-Score — wenn die Schlange gut genug
  überlebt um im Kreis zu fahren, steigt game_len obwohl die Policy schlechter
  wird. Run 016 Seed 15 illustriert das deutlich: best.mlp (Periodic 69.7)
  schlägt final.mlp (Periodic 57.6) erheblich.
- **bench_mlp ohne Rebuild**: Ein neues `bench_mlp`-Example ermöglicht das
  Benchmark einer beliebigen `.mlp`-Datei on the fly (`cargo run --release -p
  snake-core --example bench_mlp training-out/.../best.mlp 50 8000 24`), ohne
  das embedded Asset auszutauschen und neu zu kompilieren.
- **Größeres Netz bringt nichts** (Run 017, 20→128→96→7, 15 751 Parameter):
  Mit 150 Iterationen und lr=1e-3 kollabiert das Training sofort (Walls 32,
  Periodic 44 vs. Baseline 37/65). Das Standard-Netz (1 639 Param) ist gut
  dimensioniert für 20 Features und 6 Aktionen — überdimensionierte Netze
  brauchen viel mehr Daten und ggf. kleinere LR.
- **Seed-Sweep als Hauptoptimierungsstrategie**: Über Run 010–016 war die
  effektivste Verbesserung stets ein breiterer Seed-Sweep. Run 015 Seed 5 und
  Run 016 Seed 15 schlagen alle Hyperparameter-Variationen (mehr Iter, andere
  eat_bonus, Zweiphasen-Training) deutlich. Erklärung: Die Verlustlandschaft hat
  viele lokale Optima; Seed bestimmt den Initialisierungspunkt und damit welches
  Optimum gefunden wird.

## Phase 9 — AlphaZero-light: die Seed-Lotterie war ein Bug (Run 022–025)

- **„Seed-Abhängigkeit" war zu großen Teilen kein Pech, sondern zwei Bugs.** Die
  Vermutung, gute Seeds seien selten-aber-echt, hielt einer gezielten
  Ein-Seed-Untersuchung nicht stand.
- **Bug 1 — Checkpoint nach Überlebenszeit**: `best.mlp` wurde über
  `avg_game_len` (Ticks/Spiel) gewählt, nicht über Score. Genau die unerwünschte
  „sicher im Kreis fahren"-Policy maximiert game_len → der Trainer speicherte
  bevorzugt den Kreis-Kollaps. Fix: Score (gefressenes Futter) aus dem Self-Play
  nach Python zurückgeben (`SelfPlayResult`) und danach auswählen.
- **Self-Play-Score genügt nicht als Auswahlkriterium**: Der mittlere
  Self-Play-Score (stochastisch, bei `--max-ticks` gedeckelt) stieg auf 72,
  während die *greedy* Policy im Benchmark Walls-Score 19 bei avg_ticks 4116
  lieferte — sie kreiste. Stochastik + Tick-Deckel verschleiern den Kollaps, den
  greedy Spiel sofort zeigt. Lehre: Checkpoints am *Deployment-Modus* (greedy)
  messen, nicht am Trainings-Modus.
- **Bug 2 — alles auf einem Board (der eigentliche Übeltäter)**: `az_selfplay`
  setzte den Board-Seed hart auf 0. Self-Play sah also **immer dasselbe Board /
  dieselbe Futter-Sequenz**, der Benchmark misst aber auf Seeds 0..N. Die Policy
  überfittete Board 0 und fuhr auf unbekannten Boards im Kreis — *das* erzeugte
  die scheinbare Seed-Lotterie. Aufgedeckt durch einen Widerspruch: Eine
  benchmark-treue Eval ergab Walls 64, der echte Benchmark Walls 18 — fürs
  *gleiche* Netz. Grund: Bei greedy Eval (Temperatur 0) ist der Aktions-RNG
  ungenutzt, also waren alle Eval-Spiele auf Board 0 identisch. Fix:
  Board-Seed pro Spiel variieren (Training divers, Eval auf 0..N wie der
  Benchmark).
- **Nach den Fixes ist „länger → schlechter" weg**: Mit Board-Vielfalt und
  greedy-Eval-Auswahl plateaut die Kurve (avg ~55–60), statt zu kollabieren;
  `eval_ticks` fällt auf ~1100 und bleibt dort (kein Kreisen). Ein *beliebiger*
  Seed (1) erreicht jetzt zuverlässig Avg ~54 statt der früheren 40–45-Lotterie.
- **Offener Rückstand ist topologiespezifisch**: Der gelernte Policy ist auf
  Walls stärker als der alte Champion (44 vs. 41), auf Periodic schwächer
  (63 vs. 76). Vermuteter Resthebel: ein **Hunger-Feature** (Ticks seit letztem
  Futter), damit der Value-Kopf „kurz vorm Verhungern" überhaupt unterscheiden
  kann — ohne das ist Kreisen für das Netz von effizientem Fressen lokal
  ununterscheidbar.

## Phase 9 — AlphaZero-light: Hunger-Feature getestet, verworfen (Run 026)

- **Negatives Ergebnis sauber dokumentiert**: Das vermutete Hunger-Feature
  (AZ-eigener 21-Input: 20 Sensoren + `ticks_since_food`/Fläche) wurde mit 6 h
  Budget (21 721 Iterationen, Seed 1) trainiert — und ist **netto schlechter**
  als Run 025 ohne Hunger: +2.2 Periodic (Rauschen), aber −7.6 Walls, −4.5 % im
  Mittel (Bench 200 Spiele).
- **Warum es nicht half**: Das Feature sollte Kreisen bekämpfen — aber der
  Board-Seed-Fix (jedes Self-Play-Spiel ein eigenes Board) hatte das Kreisen
  schon beseitigt. Hunger löste also kein offenes Problem, sondern machte die
  Policy nur aggressiver beim Futter-Ansteuern: gut auf dem Torus, tödlich an
  Wänden. Lehre: Erst prüfen, ob das Zielproblem überhaupt noch existiert, bevor
  man ein Feature dagegen baut.
- **6 h ≈ 3 h**: Die greedy-Eval plateaut nach ~2,5 h; weitere 12 000
  Iterationen bringen kein neues Maximum. Immerhin kein Reward-Kollaps mehr
  (dank der Run-024-Fixes) — nur Sättigung. Großes Compute-Budget ersetzt keinen
  besseren Lernhebel.
- **`--max-hours` als Trainer-Regler**: Wall-Clock-Budget statt fixer
  Iterationszahl; da best.mlp laufend beim besten Eval gesichert wird, liefert
  ein zeitbegrenzter Lauf trotzdem das beste Netz.
- **Auswahl-Bias bemerkt**: Eval-Mittel `(W+P)/2` mit optimistischem
  Periodic-Eval zieht die Checkpoint-Wahl zu Periodic-lastigen Netzen. Für
  künftige balancierte Auswahl: `min(W,P)` oder Eval bei 8000 Ticks.
- **6 h ohne Hunger (Run 027): Plateau bestätigt, kleiner Gewinn**: Dieselbe
  Konfiguration ohne Hunger, aber mit 6 h Budget (19 826 Iterationen) findet das
  beste Netz schon nach ~15 min (iter 815) und verbessert sich danach nicht mehr
  im Eval-Mittel. Das Netz schlägt Run 025 leicht (Avg 62.09 vs 60.33, +8 %
  Periodic, −5 % Walls) und wurde deployed. Lehre: Bei früher Sättigung bringt
  10–40× mehr Compute fast nichts — der Hebel ist die Auswahl-/Reward-Struktur,
  nicht die Laufzeit.
- **Policy driftet zum Spezialisten**: Über die 6 h wird der *finale* Checkpoint
  ein extremer Walls-Spezialist (Walls 63.1, aber Periodic nur 57.5), während
  das deployte frühe best.mlp ausgewogener ist (45.7/78.4). Dasselbe Seed,
  dieselbe Reward — nur längeres Training kippt die Balance. Ein einzelnes Netz,
  das *beide* Topologien dominiert, scheint mit dieser Architektur/Reward schwer
  erreichbar; ein topologiespezifisches oder per-`min(W,P)` ausgewähltes Netz
  wäre der nächste Schritt.
