# E2E-Smoke-Test (Browser/WASM)

Ein einzelner, manuell startbarer Playwright-Test für die Schicht, die die
nativen Tests nicht abdecken: WASM-Bootstrap, Canvas, URL-Parameter,
Browser-Tastatur-Events und localStorage-Persistenz.

Für UI-*Zustände* (Themes, Dialoge) gibt es stattdessen deterministische
Snapshot-Tests mit `egui_kittest` direkt in `cargo test` — siehe
`crates/snake-app/src/snapshot_tests.rs` und Skill `/test-debug`.

## Ausführen

```bash
# einmalig
cd e2e && npm install && npx playwright install chromium

# Dev-Server starten (eigenes Terminal)
cd crates/snake-app && trunk serve

# Test
node e2e/smoke.mjs
```

Exit-Code 0 = OK. Der Test ist bewusst tastaturgetrieben (keine
Pixel-Koordinaten) und prüft per localStorage-Inhalt statt per
Screenshot-Vergleich — Browser-Screenshots sind als Regressionstest zu
fragil (Font-/Rendering-Unterschiede), als Debug-Werkzeug aber weiterhin
nützlich (vgl. `/test-debug`).
