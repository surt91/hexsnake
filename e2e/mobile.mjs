// End-to-end check that the board stays fully visible on a narrow (mobile)
// viewport. A HUD row wider than the screen used to inflate the board's
// painter, sizing it larger than the viewport and pushing it off the sides.
//
// The assertion is geometric, not a pixel-diff: it decodes the screenshot
// and checks the board's horizontal extent is centered and inset from both
// edges. That is robust to font/rendering differences (unlike a snapshot),
// while still catching a board that bleeds off-screen.
//
// Prerequisites: `trunk serve` running (see e2e/README.md), then:
//   node e2e/mobile.mjs

import { inflateSync } from 'node:zlib';
import { chromium } from 'playwright';

const BASE = process.env.HEXSNAKE_URL ?? 'http://127.0.0.1:8080';
// iPhone-class portrait width: narrow enough that the full HUD row does not
// fit, which is exactly the condition that used to break the board layout.
const VIEWPORT = { width: 390, height: 844 };

const fail = (msg) => {
  console.error(`FAIL: ${msg}`);
  process.exitCode = 1;
};

// Minimal PNG → RGBA decoder for the 8-bit, non-interlaced screenshots
// Playwright produces. Supports color types 2 (RGB) and 6 (RGBA).
function decodePng(buf) {
  let pos = 8; // skip signature
  let width = 0;
  let height = 0;
  let colorType = 0;
  const idat = [];
  while (pos < buf.length) {
    const len = buf.readUInt32BE(pos);
    const type = buf.toString('ascii', pos + 4, pos + 8);
    const data = buf.subarray(pos + 8, pos + 8 + len);
    if (type === 'IHDR') {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      colorType = data[9];
    } else if (type === 'IDAT') {
      idat.push(data);
    } else if (type === 'IEND') {
      break;
    }
    pos += 12 + len; // length + type + data + CRC
  }
  const channels = colorType === 6 ? 4 : 3;
  const raw = inflateSync(Buffer.concat(idat));
  const stride = width * channels;
  const rgba = Buffer.alloc(width * height * 4);
  const prev = Buffer.alloc(stride);
  const cur = Buffer.alloc(stride);
  let src = 0;
  const paeth = (a, b, c) => {
    const p = a + b - c;
    const pa = Math.abs(p - a);
    const pb = Math.abs(p - b);
    const pc = Math.abs(p - c);
    return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
  };
  for (let y = 0; y < height; y++) {
    const filter = raw[src++];
    for (let i = 0; i < stride; i++) {
      const x = raw[src++];
      const a = i >= channels ? cur[i - channels] : 0;
      const b = prev[i];
      const c = i >= channels ? prev[i - channels] : 0;
      let v;
      switch (filter) {
        case 1: v = x + a; break;
        case 2: v = x + b; break;
        case 3: v = x + ((a + b) >> 1); break;
        case 4: v = x + paeth(a, b, c); break;
        default: v = x;
      }
      cur[i] = v & 0xff;
    }
    for (let px = 0; px < width; px++) {
      const o = (y * width + px) * 4;
      rgba[o] = cur[px * channels];
      rgba[o + 1] = cur[px * channels + 1];
      rgba[o + 2] = cur[px * channels + 2];
      rgba[o + 3] = channels === 4 ? cur[px * channels + 3] : 255;
    }
    cur.copy(prev);
  }
  return { width, height, rgba };
}

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: VIEWPORT, hasTouch: true });
const pageErrors = [];
page.on('pageerror', (e) => pageErrors.push(String(e)));

try {
  await page.goto(BASE + '?seed=42', { timeout: 15000 });
} catch {
  console.error(`Cannot reach ${BASE} — is \`trunk serve\` running?`);
  process.exit(2);
}
await page.waitForTimeout(3500); // WASM bootstrap

// Start the game so the board is on screen, then grab a running frame.
await page.keyboard.press('Enter');
await page.waitForTimeout(1200);

const { width, height, rgba } = decodePng(await page.screenshot());
const at = (x, y) => {
  const o = (y * width + x) * 4;
  return [rgba[o], rgba[o + 1], rgba[o + 2]];
};

// Three brightness tiers are on screen (default theme): the light egui
// frame (~248), the dark board background (~24) and, in between, the board
// content — wall frame (~126), snake and food. A mid-brightness band picks
// out that content and ignores both backgrounds, which is what defines the
// board's visible extent.
const isBoard = (x, y) => {
  const [r, g, b] = at(x, y);
  const lum = (r + g + b) / 3;
  return lum >= 60 && lum <= 210;
};

// Scan a band across the vertical middle (well below the HUD), find the
// horizontal extent of board content.
let minX = width;
let maxX = -1;
let count = 0;
for (let y = Math.floor(height * 0.45); y < Math.floor(height * 0.55); y++) {
  for (let x = 0; x < width; x++) {
    if (isBoard(x, y)) {
      if (x < minX) minX = x;
      if (x > maxX) maxX = x;
      count++;
    }
  }
}

if (count < 50) {
  fail(`no board content detected in the middle band (count=${count})`);
} else {
  // Inset from both edges → not clipped on either side.
  if (minX < 2) fail(`board reaches the left edge (minX=${minX})`);
  if (maxX > width - 3) fail(`board reaches the right edge (maxX=${maxX}, width=${width})`);
  // Actually fills most of the width → it is the board, not a stray glyph.
  if (maxX - minX < width * 0.5) {
    fail(`board too narrow to be fully shown (span=${maxX - minX}, width=${width})`);
  }
  // Roughly centred.
  const center = (minX + maxX) / 2;
  if (Math.abs(center - width / 2) > width * 0.15) {
    fail(`board not centred (center=${center.toFixed(0)}, width=${width})`);
  }
}

if (pageErrors.length > 0) {
  fail(`page errors: ${pageErrors.join(' | ').slice(0, 500)}`);
}

await browser.close();
console.log(process.exitCode ? 'mobile test FAILED' : 'mobile test OK');
