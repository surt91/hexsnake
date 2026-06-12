---
name: test-debug
description: Test- und Debugstrategien für HexSnake - Bugs deterministisch reproduzieren, UI-Zustände als Screenshot festhalten (egui_kittest, Playwright CLI), Logging in nativ und WASM. Nutzen bei Bughunting, beim Schreiben von Tests oder wenn ein visueller Zustand geprüft/dokumentiert werden soll.
---

# Test- & Debugstrategien

## Grundprinzip: erst deterministisch reproduzieren

Das Spiel ist vollständig durch **Seed + Inputliste** bestimmt. Jeder Bug
sollte zuerst auf diese Form gebracht werden, dann ist er headless als
Unit-Test reproduzierbar — debugge nie länger im Browser, als nötig ist, um
Seed + Inputs zu extrahieren.

Triage: Logikfehler (falsches Verhalten bei gleichem Input) → `snake-core`,
headless. Darstellungs-/Eingabefehler (Logik korrekt, Anzeige falsch) →
`snake-app`, Snapshot oder Browser.

## Logik testen (snake-core, headless)

- **Regressionstest aus Bug**: Seed + Inputs als Test einfrieren, erwarteten
  Zustand assert-en. Schnellste und stabilste Testform — Default.
- **Ganze Partien**: Strategie spielt headless N Partien; assert auf
  Invarianten (kein Schlangensegment doppelt, Futter nie auf der Schlange,
  Score == Länge − Startlänge).
- **Property-Tests** (`proptest`) für die Hex-Mathematik: axial↔offset-
  Roundtrip, `neighbor(opposite(d))` ist Inverse, Torus-Distanz symmetrisch
  und ≤ ungewrappte Distanz.
- **Determinismus-Test**: gleicher Seed + gleiche Inputs zweimal ausführen,
  Zustände müssen identisch sein (schützt Replays und Server-Verifikation).

## UI-Zustände: Snapshot-Tests mit egui_kittest

Für „sieht Zustand X richtig aus?" ist `egui_kittest` (offizielles
egui-Test-Harness, Rendering via wgpu) das Mittel der Wahl — kein Browser
nötig, beliebige Zustände direkt konstruierbar:

1. `GameState` im Test gezielt bauen (z. B. Schlange am Rand im
   periodischen Modus, Game-Over-Screen).
2. Harness rendert die App/das Panel, `harness.snapshot("name")` vergleicht
   gegen ein abgelegtes Referenzbild.
3. Referenzbilder aktualisieren: `UPDATE_SNAPSHOTS=1 cargo test -p snake-app`
   (Env-Var-Name bei Einrichtung gegen die egui_kittest-Doku prüfen).
   Geänderte Snapshots im Diff ansehen, bevor sie committet werden.

Die erzeugten PNGs eignen sich auch als Screenshot-Quelle, um dem Nutzer
einen Zustand zu zeigen: Snapshot-Test schreiben, Bild aus dem
Snapshot-Verzeichnis anhängen.

## Browser: Screenshots & Steuerung mit Playwright CLI

Voraussetzung: laufender Dev-Server (`/run-web`), einmalig
`npx playwright install chromium`.

Schneller Screenshot:

```bash
npx playwright screenshot --wait-for-timeout=2000 \
  http://127.0.0.1:8080 /tmp/hexsnake.png
```

(`--wait-for-timeout` gibt dem WASM-Modul Zeit zu laden; bei leerem/weißem
Bild Timeout erhöhen.)

Spiel steuern und Zustand herstellen (Node-Skript mit Playwright):

```js
// node /tmp/drive.mjs  — Partie starten, Züge spielen, Screenshot
import { chromium } from 'playwright';
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1024, height: 768 } });
page.on('console', m => console.log('[console]', m.text()));   // WASM-Logs!
await page.goto('http://127.0.0.1:8080');
await page.waitForTimeout(2000);
await page.locator('canvas').click();          // Canvas fokussieren, sonst
for (const k of ['w', 'e', 'd']) {             // kommen Tasten nicht an
  await page.keyboard.press(k);
  await page.waitForTimeout(300);
}
await page.screenshot({ path: '/tmp/state.png' });
await browser.close();
```

- `page.on('console', ...)` ist im WASM-Build der wichtigste Debug-Kanal —
  Panics und Logs landen in der Browser-Konsole, nicht auf stdout.
- Für reproduzierbare Browser-Zustände lohnt ein Debug-Feature in der App:
  Seed (und optional Inputliste) per URL-Parameter, z. B. `?seed=42` —
  damit zeigen Screenshots verschiedener Läufe denselben Zustand. Bei
  Bedarf implementieren (kleiner Eingriff, nur im Debug-/Query-Pfad).

## Logging

- **Nativ**: `log`-Crate + `env_logger`; aktivieren mit
  `RUST_LOG=snake_core=debug cargo run -p snake-app`.
- **WASM**: Logger-Backend für die Browser-Konsole einrichten (z. B.
  `console_log` oder `tracing-wasm`) und `console_error_panic_hook`
  aktivieren, damit Panics einen lesbaren Stacktrace liefern — beides
  Standard im eframe-Template.
- Tick-genaues Debugging in core: statt println-Kaskaden den Zustand bei
  Bedarf als kompakten String dumpen (`Display` für `GameState` mit
  ASCII-Hexfeld) — hilft in Test-Failures wie im Log.

## KI-Strategien debuggen

- **Debug-Overlay** (ab Phase 5) einschalten: geplanter Pfad, Flood-Fill-
  Heatmap, Bewertung je Richtung — meist sieht man dort sofort, *was* die
  Strategie falsch einschätzt.
- Verdächtige Situation als Unit-Test: `GameState` konstruieren,
  `strategy.next_move(&state)` assert-en. Strategien sind UI-frei, das
  geht immer headless.
- Bei „Strategie spielt im periodischen Modus schlecht": fast immer wird
  irgendwo die euklidische statt der Torus-Distanz benutzt — zuerst dort
  suchen (vgl. Risiken in plan/01_snake.md).
