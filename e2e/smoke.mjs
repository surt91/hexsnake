// End-to-end smoke test for the WASM build. Covers exactly the layer the
// native egui_kittest tests cannot: canvas bootstrap, URL parameters,
// keyboard events in the browser and localStorage persistence.
//
// Prerequisites: `trunk serve` running (see e2e/README.md), then:
//   node e2e/smoke.mjs
//
// Keyboard-driven only — no pixel coordinates, so menu layout changes
// don't break it.

import { chromium } from 'playwright';

const BASE = process.env.HEXSNAKE_URL ?? 'http://127.0.0.1:8080';
// Deterministic run on the default 24×18 board: eats twice, then crashes
// (generate fresh scripts with: cargo run -p snake-core --example greedy_inputs).
const QUERY = '?seed=42&inputs=assaaaaaaaaqqqwwwwwwwwwwwwwww';

const fail = (msg) => {
  console.error(`FAIL: ${msg}`);
  process.exitCode = 1;
};

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1024, height: 768 } });
const pageErrors = [];
page.on('pageerror', (e) => pageErrors.push(String(e)));

try {
  await page.goto(BASE + QUERY, { timeout: 15000 });
} catch {
  console.error(`Cannot reach ${BASE} — is \`trunk serve\` running?`);
  process.exit(2);
}
await page.waitForTimeout(3500); // WASM bootstrap

// Menu → start the scripted game, let it play to game over (~5 s).
await page.keyboard.press('Enter');
await page.waitForTimeout(7000);

// Highscore dialog grabs focus: type a name, Enter saves.
const name = `E2E${Date.now() % 10000}`;
await page.keyboard.type(name);
await page.keyboard.press('Enter');

// Wait past the eframe auto-save interval (5 s), then verify persistence.
await page.waitForTimeout(7000);
const stored = await page.evaluate(() => JSON.stringify(Object.entries(localStorage)));
if (!stored.includes(name)) {
  fail(`highscore name "${name}" not found in localStorage after save`);
}

// Survives a reload.
await page.reload();
await page.waitForTimeout(3000);
const storedAfter = await page.evaluate(() => JSON.stringify(Object.entries(localStorage)));
if (!storedAfter.includes(name)) {
  fail(`highscore name "${name}" lost after reload`);
}

if (pageErrors.length > 0) {
  fail(`page errors: ${pageErrors.join(' | ').slice(0, 500)}`);
}

await browser.close();
console.log(process.exitCode ? 'smoke test FAILED' : 'smoke test OK');
