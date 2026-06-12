---
name: run-web
description: HexSnake im Browser bauen und starten (trunk/WASM) oder nativ ausführen. Nutzen, wenn das Spiel gestartet, angesehen oder ein UI-Verhalten im Browser verifiziert werden soll.
---

# App starten

## Nativ (schnellste Dev-Schleife, für die meisten UI-Checks ausreichend)

```bash
cargo run -p snake-app
```

## Browser (WASM)

```bash
cd crates/snake-app
trunk serve --open=false
```

- Läuft dann auf `http://127.0.0.1:8080`; trunk rebuildet bei Dateiänderungen
  automatisch.
- Als Hintergrundprozess starten und die Ausgabe auf Build-Fehler prüfen;
  ein erfolgreicher Build loggt `serving static assets`.
- Release-Build für Deployment: `trunk build --release` (Ausgabe in `dist/`).

## Voraussetzungen (einmalig, bei Fehlern zuerst prüfen)

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

## Typische Stolpersteine

- `index.html` muss im `snake-app`-Crate-Root liegen (trunk-Konvention).
- Browser-Persistenz (localStorage) funktioniert über die eframe
  `Storage`-API nur, wenn in `index.html`/`main` eine stabile App-ID gesetzt
  ist.
- Im WASM-Build kein `std::thread`, kein blockierendes IO; HTTP nur über
  `ehttp`.
