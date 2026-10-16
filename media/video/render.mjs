// render.mjs — draw one cut of the intro video, frame by frame, with a
// headless Chrome. Called by render.sh beside it, which prepares the work
// directory first (packages, stills, clip frames); run that, not this.
//
// What it does: serves composition.{html,css,js} (beside it), the fonts and
// the work directory to a headless Chrome under a made-up origin (no port is
// opened, no window is shown), asks the page for the cut's length, refuses a
// character the bundled fonts cannot draw, then for every frame calls
// WM.seek(t) and writes a 1920×1080 PNG. With --vtt it also writes the cut's
// captions as WebVTT, from the same data the frames are drawn from.
//
// Usage: node render.mjs --work <dir> --chrome <path> --cut <main|loop>
//                        --out <frames dir> [--counts clip:n,...] [--jobs 4]
//                        [--vtt <file>] [--only <frame>,<frame>,...]
// Frames are numbered from 0 here and written from 00001.png, as ffmpeg
// reads them; --only draws just those frames (to look at one moment).
import { createRequire } from "node:module";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, extname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const args = {};
for (let i = 2; i < process.argv.length; i += 2) {
  const key = process.argv[i].replace(/^--/, "");
  args[key] = process.argv[i + 1];
}
for (const need of ["work", "chrome", "cut", "out"]) {
  if (!args[need]) {
    console.error(`render.mjs: --${need} is required`);
    process.exit(2);
  }
}
const work = resolve(args.work);
const jobs = Math.max(1, Number(args.jobs || 4));
const require = createRequire(join(work, "node_modules", "noop.js"));
const { chromium } = require("playwright-core");

const ORIGIN = "https://wipemark.video.invalid";
const TYPES = { ".html": "text/html", ".css": "text/css", ".js": "text/javascript", ".png": "image/png", ".woff2": "font/woff2" };

// The only files the page may read: the composition, two font packages and
// the work directory. Anything else is a 404, so nothing from the network or
// from elsewhere on the disk can reach a frame.
function local(pathname) {
  const inside = (root, rel) => {
    const full = resolve(root, rel);
    return full.startsWith(root + sep) ? full : null;
  };
  if (/^\/composition\.(html|css|js)$/.test(pathname)) return join(here, pathname.slice(1));
  if (pathname.startsWith("/fonts/")) {
    const file = pathname.slice("/fonts/".length);
    const pkg = file.startsWith("jetbrains-mono-") ? "jetbrains-mono" : "inter";
    return inside(join(work, "node_modules", "@fontsource", pkg, "files"), file);
  }
  if (pathname.startsWith("/work/")) return inside(work, decodeURIComponent(pathname.slice("/work/".length)));
  return null;
}

// The Unicode ranges of the fonts' latin subsets (@fontsource unicode.json).
const LATIN = [
  [0x0000, 0x00ff], [0x0131, 0x0131], [0x0152, 0x0153], [0x02bb, 0x02bc], [0x02c6, 0x02c6],
  [0x02da, 0x02da], [0x02dc, 0x02dc], [0x2000, 0x206f], [0x20ac, 0x20ac], [0x2122, 0x2122],
  [0x2191, 0x2191], [0x2193, 0x2193], [0x2212, 0x2212], [0x2215, 0x2215],
];
const drawable = (cp) => LATIN.some(([a, b]) => cp >= a && cp <= b);

function vttTime(t) {
  const ms = Math.round(t * 1000);
  const h = Math.floor(ms / 3600000);
  const m = Math.floor((ms % 3600000) / 60000);
  const s = Math.floor((ms % 60000) / 1000);
  const r = ms % 1000;
  const two = (n) => String(n).padStart(2, "0");
  return `${two(h)}:${two(m)}:${two(s)}.${String(r).padStart(3, "0")}`;
}

async function openPage(browser) {
  const context = await browser.newContext({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  await page.route("**/*", async (route) => {
    const url = new URL(route.request().url());
    const file = url.origin === ORIGIN ? local(url.pathname) : null;
    if (!file) return route.fulfill({ status: 404, body: "" });
    try {
      const body = readFileSync(file);
      return route.fulfill({ status: 200, body, headers: { "content-type": TYPES[extname(file)] || "application/octet-stream" } });
    } catch {
      return route.fulfill({ status: 404, body: "" });
    }
  });
  const query = new URLSearchParams({ cut: args.cut, counts: args.counts || "" });
  await page.goto(`${ORIGIN}/composition.html?${query}`);
  await page.evaluate(() => window.WM.ready());
  if (errors.length) throw new Error("the composition failed: " + errors.join("; "));
  return { page, errors };
}

const browser = await chromium.launch({
  executablePath: args.chrome,
  headless: true,
  args: ["--force-color-profile=srgb", "--disable-lcd-text", "--font-render-hinting=none", "--hide-scrollbars", "--mute-audio"],
});
try {
  const first = await openPage(browser);
  const { fps, duration } = await first.page.evaluate(() => ({ fps: window.WM.fps, duration: window.WM.duration }));

  const text = await first.page.evaluate(() => window.WM.text());
  const missing = [...new Set([...text].filter((ch) => !drawable(ch.codePointAt(0))))];
  if (missing.length) {
    throw new Error("characters the bundled fonts do not draw: " + missing.map((c) => "U+" + c.codePointAt(0).toString(16).toUpperCase().padStart(4, "0")).join(" "));
  }

  // A post cropped to a square keeps the centre 1080 px: every word must
  // fit there. (The loop has no words.)
  if (args.cut === "main") {
    const wide = await first.page.evaluate(() => window.WM.tooWide());
    if (wide.length) throw new Error("outside the centre square (x 430–1490): " + wide.map((w) => `"${w.text}" (${w.left}–${w.right})`).join("; "));
  }

  if (args.vtt) {
    const cues = await first.page.evaluate(() => window.WM.cues());
    let lastKicker = "";
    const blocks = cues.map((c) => {
      let body = c.text;
      if (c.kicker && c.kicker !== lastKicker) body = c.kicker + "\n" + body;
      if (c.kicker) lastKicker = c.kicker;
      return `${vttTime(c.start)} --> ${vttTime(c.end)}\n${body}`;
    });
    writeFileSync(args.vtt, "WEBVTT\n\n" + blocks.join("\n\n") + "\n");
    console.log(`wrote ${args.vtt}: ${cues.length} cues`);
  }

  const total = Math.round(duration * fps);
  const frames = args.only != null ? args.only.split(",").map(Number) : [...Array(total).keys()];
  mkdirSync(args.out, { recursive: true });
  const pages = [first];
  for (let i = 1; i < Math.min(jobs, frames.length); i++) pages.push(await openPage(browser));

  let done = 0;
  const started = Date.now();
  await Promise.all(
    pages.map(async ({ page, errors }, worker) => {
      for (let k = worker; k < frames.length; k += pages.length) {
        const f = frames[k];
        await page.evaluate((t) => window.WM.seek(t), f / fps);
        const png = await page.screenshot({ type: "png", clip: { x: 0, y: 0, width: 1920, height: 1080 }, animations: "disabled", caret: "initial", scale: "css" });
        if (errors.length) throw new Error(`frame ${f}: ` + errors.join("; "));
        writeFileSync(join(args.out, String(f + 1).padStart(5, "0") + ".png"), png);
        done++;
        if (done % 150 === 0) console.log(`  ${args.cut}: ${done}/${frames.length} frames, ${((Date.now() - started) / 1000).toFixed(0)} s`);
      }
    }),
  );
  console.log(`${args.cut}: ${frames.length} frames (${duration.toFixed(2)} s at ${fps} fps) in ${((Date.now() - started) / 1000).toFixed(0)} s`);
} finally {
  await browser.close();
}
