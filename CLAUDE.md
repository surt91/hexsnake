# HexSnake

Snake auf einem hexagonalen Gitter (Flat-Top-Hexagons, 6 Richtungen via
QWEASD), gebaut mit Rust + egui/eframe, lauffähig nativ und im Browser
(WASM via trunk). Optionaler axum-Server für globale Highscores —
das Spiel muss **offline immer voll funktionieren**.

## Maßgebliche Dokumente

- `docs/concept.md` — Spielkonzept, Hex-Geometrie, alle Strategien und
  Features, getroffene Entscheidungen (§6). Bei Designfragen zuerst hier
  nachsehen.
- `plan/01_snake.md` — der Umsetzungsplan, **Source of Truth für den
  Arbeitsstand**. Phasen strikt in Reihenfolge abarbeiten; erledigte
  Checkboxen (`- [x]`) direkt im Plan abhaken. Eine Phase gilt erst als
  fertig, wenn ihr „Done wenn"-Kriterium erfüllt ist.

## Architektur-Leitplanken

- Cargo-Workspace: `crates/snake-core` (Logik), `crates/snake-app` (egui),
  später `crates/snake-train` (NN) und `crates/snake-server` (axum).
- `snake-core` bleibt **frei von UI-Abhängigkeiten** und vollständig
  **deterministisch**: seedbarer RNG (`Pcg32`), keine Abhängigkeit von
  HashMap-Iterationsreihenfolge, keine Wall-Clock. Gleicher Seed + gleiche
  Inputs ⇒ identischer Spielverlauf (Grundlage für Replays und
  Server-Verifikation).
- Alles in `snake-core` muss nach `wasm32-unknown-unknown` kompilieren.
- Spiellogik-Änderungen brauchen Unit-Tests in `snake-core`; UI-Code wird
  nicht über Tests abgedeckt.
- Strategien implementieren das `Strategy`-Trait und arbeiten nur auf dem
  `GameState` — kein UI-Zugriff.

## Konventionen

- **Conventional Commits, vom Agenten angelegt**: Nach jeder abgeschlossenen,
  in sich konsistenten Arbeitseinheit (typisch: ein Checkbox-Punkt aus dem
  Plan) committet der Agent selbstständig mit `feat:` / `fix:` / `docs:` /
  `test:` / `refactor:` / `chore:` und optionalem Scope, z. B.
  `feat(core): add torus distance for periodic boundaries`.
  Commit-Subject auf Englisch, Imperativ, ≤ 72 Zeichen. Vor jedem Commit
  die Checks laufen lassen (siehe Skill `/check`).
- Code, Bezeichner und Code-Kommentare auf Englisch; Nutzer-Dokumentation
  (`docs/`, `plan/`) auf Deutsch.
- `cargo fmt` ist verbindlich, Clippy-Warnungen werden behoben statt
  allowed (begründete Ausnahmen mit Kommentar).

## Befehle

```bash
cargo test --workspace                                  # Tests
cargo clippy --workspace --all-targets -- -D warnings   # Lint
cargo fmt --all                                         # Format
cargo run -p snake-app                                  # Nativ (schnellste Dev-Schleife)
trunk serve                                             # Browser-Build, aus crates/snake-app/
cargo check -p snake-app --target wasm32-unknown-unknown # WASM-Kompilierbarkeit prüfen
```

Voraussetzungen einmalig: `rustup target add wasm32-unknown-unknown` und
`cargo install trunk`.

## Hex-Spickzettel

- Flat-Top-Hexagons; Richtungen N, NE, SE, S, SW, NW ↔ Tasten W, E, D, S, A, Q.
- Intern axiale Koordinaten `(q, r)`; das Spielfeld ist ein Rechteck in
  Offset-Koordinaten `(col, row)` — periodischer Rand wrappt dort per Modulo.
- Bei periodischen Rändern immer die Torus-Distanz verwenden (Minimum über
  die Wrap-Varianten), sonst spielen die Strategien falsch.
- Referenz für Hex-Mathematik: Red Blob Games, „Hexagonal Grids".
