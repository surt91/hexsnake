# E2E-Tests (Browser/WASM)

Manuell startbare Playwright-Skripte für die Schicht, die die nativen Tests
nicht abdecken: WASM-Bootstrap, Canvas, URL-Parameter, Browser-Tastatur-Events
und localStorage-Persistenz.

- `smoke.mjs` — Happy-Path: scriptgesteuerte Partie bis Game Over, Highscore
  speichern, Persistenz über einen Reload prüfen.
- `mobile.mjs` — lädt das Spiel in einem schmalen Mobil-Viewport (390×844) und
  stellt sicher, dass das **Spielfeld vollständig sichtbar** ist (zentriert,
  beidseitig vom Rand abgesetzt). Schützt gegen die Regression, dass eine zu
  breite HUD-Zeile das Brett größer als den Viewport macht und an den Seiten
  abschneidet.

Für UI-*Zustände* (Themes, Dialoge) gibt es stattdessen deterministische
Snapshot-Tests mit `egui_kittest` direkt in `cargo test` — siehe
`crates/snake-app/src/snapshot_tests.rs` und Skill `/test-debug`.

## Ausführen

```bash
# einmalig
cd e2e && npm install && npx playwright install chromium

# Dev-Server starten (eigenes Terminal)
cd crates/snake-app && trunk serve

# Tests
node e2e/smoke.mjs
node e2e/mobile.mjs
```

Exit-Code 0 = OK. `smoke.mjs` ist bewusst tastaturgetrieben (keine
Pixel-Koordinaten) und prüft per localStorage-Inhalt statt per
Screenshot-Vergleich — Browser-Screenshots sind als Regressionstest zu
fragil (Font-/Rendering-Unterschiede), als Debug-Werkzeug aber weiterhin
nützlich (vgl. `/test-debug`). `mobile.mjs` wertet zwar einen Screenshot aus,
prüft aber nur eine *geometrische* Eigenschaft (horizontale Ausdehnung des
Bretts), nicht den exakten Pixelinhalt — daher robust gegen Font-/Rendering-
Unterschiede.
