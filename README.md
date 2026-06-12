# HexSnake

Snake auf einem hexagonalen Gitter (Flat-Top-Hexagons, 6 Richtungen via
QWEASD), gebaut mit Rust + egui/eframe. Läuft nativ und im Browser (WASM).

## Bauen & Ausführen

Voraussetzungen (einmalig):

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

Nativ (schnellste Dev-Schleife):

```bash
cargo run -p snake-app
```

Im Browser (mit Hot-Reload):

```bash
cd crates/snake-app
trunk serve --open
```

Release-Build für statisches Hosting:

```bash
cd crates/snake-app
trunk build --release
# Ergebnis in crates/snake-app/dist/
```

## Entwicklung

```bash
cargo test --workspace                                   # Tests
cargo clippy --workspace --all-targets -- -D warnings    # Lint
cargo fmt --all                                          # Format
cargo check -p snake-app --target wasm32-unknown-unknown # WASM-Check
```

Architektur und Konzept: siehe [`docs/concept.md`](docs/concept.md),
Umsetzungsplan: [`plan/01_snake.md`](plan/01_snake.md).
