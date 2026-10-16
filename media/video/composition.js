// The intro video's composition: what is on screen at every moment of the
// two cuts, as a function of time. render.mjs (beside it) loads
// composition.html?cut=<main|loop>&counts=<clip:frames,...>, calls
// WM.ready() once, then WM.seek(t) for every frame and takes a screenshot.
//
// Every word a viewer reads is in this file, and each must be true of
// Wipemark 0.1.0 (README.md and site/index.html are the reference). The
// pictures are only Wipemark's own windows (media/out/clips/ and
// media/out/screenshots/, copied into the work directory by render.sh) and
// shapes drawn here. `WM.cues()` hands the captions to render.mjs, which
// writes them as WebVTT, so the .vtt and the picture cannot disagree.
"use strict";

(function () {
  const FPS = 30;
  const params = new URLSearchParams(location.search);
  const CUT = params.get("cut") || "main";
  const COUNTS = {};
  for (const pair of (params.get("counts") || "").split(",")) {
    const [clip, n] = pair.split(":");
    if (clip) COUNTS[clip] = Number(n);
  }

  // ── Small tools ─────────────────────────────────────────────────────
  const $ = (tag, cls, parent, html) => {
    const el = document.createElement(tag);
    if (cls) el.className = cls;
    if (html != null) el.innerHTML = html;
    if (parent) parent.appendChild(el);
    return el;
  };
  const clamp = (x, a = 0, b = 1) => Math.min(b, Math.max(a, x));
  const lerp = (a, b, u) => a + (b - a) * u;
  const E = {
    lin: (u) => u,
    out: (u) => 1 - Math.pow(1 - u, 3),
    in: (u) => u * u * u,
    io: (u) => (u < 0.5 ? 4 * u * u * u : 1 - Math.pow(-2 * u + 2, 3) / 2),
    sine: (u) => (1 - Math.cos(Math.PI * u)) / 2,
  };
  const prog = (t, a, b, e = E.io) => e(clamp((t - a) / (b - a)));
  // A track of keyframes [[time, value], ...]; a value is a number or an
  // object of numbers. Before the first key it is the first value, after the
  // last the last.
  function track(t, keys, e = E.io) {
    if (t <= keys[0][0]) return keys[0][1];
    for (let i = 1; i < keys.length; i++) {
      const [t1, v1] = keys[i];
      if (t <= t1) {
        const [t0, v0] = keys[i - 1];
        const u = e(clamp((t - t0) / (t1 - t0)));
        if (typeof v0 === "number") return lerp(v0, v1, u);
        const out = {};
        for (const k in v0) out[k] = lerp(v0[k], v1[k], u);
        return out;
      }
    }
    return keys[keys.length - 1][1];
  }
  const px = (n) => n.toFixed(2) + "px";
  const TICK =
    '<svg viewBox="0 0 34 34" width="100%" height="100%"><path d="M10 17.5l4.6 4.6L24.5 12" fill="none" stroke="#fff" stroke-width="3.4" stroke-linecap="round" stroke-linejoin="round"/></svg>';

  // ── Images: every one is decoded before its frame is taken ──────────
  const pending = [];
  function setSrc(img, src) {
    if (img.dataset.src === src) return;
    img.dataset.src = src;
    img.src = src;
    pending.push(
      img.decode().catch(() => {
        throw new Error("cannot load " + src);
      }),
    );
  }
  const still = (name) => "/work/stills/" + name;
  function clipFrame(img, clip, ct) {
    const n = COUNTS[clip];
    if (!n) throw new Error("no frame count for clip " + clip);
    const i = clamp(Math.round(ct * FPS) + 1, 1, n);
    setSrc(img, "/work/frames/" + clip + "/" + String(i).padStart(5, "0") + ".png");
  }

  // ── Scenes ──────────────────────────────────────────────────────────
  // A scene is visible from `start` to `end`, fading in over `fin` seconds
  // and out over `fout`. Later scenes are drawn over earlier ones, so a
  // window that dissolves into the next one never shows the background.
  const scenes = [];
  const scenesRoot = document.getElementById("scenes");
  function scene(start, end, opts = {}) {
    const el = $("div", "scene", scenesRoot);
    const s = {
      start,
      end,
      el,
      fin: opts.fin ?? 0.45,
      fout: opts.fout ?? 0.45,
      update: () => {},
    };
    scenes.push(s);
    return s;
  }
  function stage(parent, full) {
    return $("div", full ? "stage full" : "stage", parent);
  }
  // One of the application's windows at its own pixel size; `cam` puts the
  // window point (cx, cy) at the frame point (ax, ay), scaled by s.
  function win(parent, w, h, radius, opts = {}) {
    const el = $("div", "win", parent);
    el.style.width = w + "px";
    el.style.height = h + "px";
    el.style.borderRadius = radius + "px";
    if (opts.transparent) el.style.background = "transparent";
    const img = $("img", null, el);
    img.style.width = (opts.imgW || w) + "px";
    img.style.height = (opts.imgH || h) + "px";
    if (opts.offX) img.style.left = -opts.offX + "px";
    if (opts.offY) img.style.top = -opts.offY + "px";
    if (opts.src) setSrc(img, opts.src);
    return { el, img, w, h, top: opts.stageTop ?? 196 };
  }
  function place(w, cam, extra = "") {
    const x = cam.ax - cam.cx * cam.s;
    const y = cam.ay - cam.cy * cam.s - w.top;
    w.el.style.transform = `translate(${px(x)}, ${px(y)}) scale(${cam.s.toFixed(5)}) ${extra}`;
  }

  // ── Captions: the band at the top of the frame ──────────────────────
  const cues = [];
  const capRoot = document.getElementById("captions");
  function cap(start, end, head, opts = {}) {
    const el = $("div", "cap" + (opts.sub ? " has-sub" : ""), capRoot);
    const kicker = opts.kicker ? $("div", "kicker", el, opts.kicker) : null;
    const h = $("div", "head", el, head);
    const sub = opts.sub ? $("div", "sub", el, opts.sub) : null;
    const c = { start, end, kicker: opts.kicker || "", head, sub: opts.sub || "", el, kEl: kicker, parts: [h, sub].filter(Boolean), band: true };
    cues.push(c);
    return c;
  }
  // Text drawn by a scene rather than in the band, for the WebVTT file.
  function said(start, end, text) {
    cues.push({ start, end, text, band: false });
  }
  function updateCaptions(t) {
    const band = cues.filter((c) => c.band);
    band.forEach((c, i) => {
      const prev = band[i - 1];
      const next = band[i + 1];
      const keepIn = prev && prev.kicker && prev.kicker === c.kicker && Math.abs(prev.end - c.start) < 0.01;
      const keepOut = next && next.kicker && next.kicker === c.kicker && Math.abs(next.start - c.end) < 0.01;
      const fin = prog(t, c.start, c.start + 0.4, E.out);
      const fout = 1 - prog(t, c.end - 0.3, c.end, E.in);
      const on = t >= c.start && t < c.end;
      c.el.style.visibility = on ? "visible" : "hidden";
      if (!on) return;
      const o = fin * fout;
      for (const p of c.parts) {
        p.style.opacity = o.toFixed(4);
        p.style.transform = `translateY(${px(lerp(14, 0, fin) - 6 * (1 - fout))})`;
      }
      if (c.kEl) {
        const ko = (keepIn ? 1 : fin) * (keepOut ? 1 : fout);
        c.kEl.style.opacity = ko.toFixed(4);
        c.kEl.style.transform = keepIn ? "none" : `translateY(${px(lerp(10, 0, fin))})`;
      }
    });
  }

  // ── Pieces shared by the cuts ───────────────────────────────────────
  // The two halves of picture-before-after.png: 1025 px each, a 6 px white
  // divider between them (x 1025–1030).
  function beforeAfter(parent, size, left, top) {
    const k = size / 1025;
    const box = $("div", "ba", parent);
    Object.assign(box.style, { left: left + "px", top: top + "px", width: size + "px", height: size + "px" });
    const mk = (offset) => {
      const layer = $("div", "half", box);
      const img = $("img", null, layer);
      Object.assign(img.style, { position: "absolute", top: "0", left: -offset * k + "px", width: 2056 * k + "px", height: 1025 * k + "px" });
      setSrc(img, still("picture-before-after.png"));
      return layer;
    };
    mk(0);
    const after = mk(1031);
    // A dashed ring around the sparkle, drawn here, not in the picture.
    const ring = $("div", null, box);
    const r = 64 * (size / 700);
    Object.assign(ring.style, {
      position: "absolute",
      left: 913 * k - r + "px",
      top: 913 * k - r + "px",
      width: 2 * r + "px",
      height: 2 * r + "px",
      borderRadius: "50%",
      border: "4px dashed #ffd84d",
      boxShadow: "0 0 0 2px rgba(0,0,0,0.12)",
    });
    const divider = $("div", "divider", box);
    const knob = $("div", "knob", divider);
    knob.innerHTML =
      '<svg viewBox="0 0 52 52" width="100%" height="100%"><path d="M21 18l-7 8 7 8M31 18l7 8-7 8" fill="none" stroke="#1f5ab8" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"/></svg>';
    return {
      box,
      // ring: 0..1 shown; wipe: 0 (before) .. 1 (after)
      set(ringO, wipe, grow) {
        after.style.clipPath = `inset(0 ${((1 - wipe) * 100).toFixed(3)}% 0 0)`;
        divider.style.left = px(wipe * size);
        divider.style.opacity = wipe > 0.001 && wipe < 0.999 ? "1" : "0";
        const pastRing = clamp((wipe * size - (913 * k - r)) / (2 * r));
        ring.style.opacity = (ringO * (1 - pastRing)).toFixed(4);
        ring.style.transform = `scale(${lerp(1.25, 1, ringO).toFixed(4)})`;
        box.style.transform = `scale(${grow.toFixed(5)})`;
      },
    };
  }

  function brandBlock(parent, y, iconSize, nameSize) {
    const icon = $("img", "brand-icon", parent);
    Object.assign(icon.style, { width: iconSize + "px", height: iconSize + "px", marginLeft: -iconSize / 2 + "px", top: y + "px" });
    setSrc(icon, still("icon.png"));
    const name = $("div", "brand-name", parent, "Wipemark");
    Object.assign(name.style, { top: y + iconSize + 26 + "px", fontSize: nameSize + "px", lineHeight: "1" });
    return { icon, name };
  }

  // ── The main cut: 72 seconds ────────────────────────────────────────
  // Each scene is written in its own time: T is when it starts, and every
  // moment inside it is T + seconds.
  function buildMain() {
    // 1. The opening. Frame 0 is already the hook: dark, a paragraph pasted
    // from an assistant, its invisible characters lighting up red with their
    // code points; then a light wipe takes them out and the gaps close.
    {
      const T = 0;
      const s = scene(T, T + 7.2, { fin: 0, fout: 0.45 });
      const MARKS = [
        ["U+200B", -1],
        ["U+2060", 0.3],
        ["U+00AD", 0.62],
        ["U+FEFF", 0.94],
      ];
      const W_MARK = 104;
      const para = (layer) => {
        const card = $("div", "hk-card", layer);
        $("div", "hk-label", card, "Pasted from an assistant");
        const mk = (i) => `<span class="mk"><i>${MARKS[i][0]}</i></span>`;
        const l1 = $("div", "hk-line", card, `The rep${mk(0)}ort is ready for re${mk(1)}view,`);
        const l2 = $("div", "hk-line", card, `and the num${mk(2)}bers are fi${mk(3)}nal.`);
        l1.style.top = "84px";
        l2.style.top = "160px";
        return { card, marks: [...card.querySelectorAll(".mk")] };
      };
      const lockup = (layer) => {
        const b = $("div", "hk-brand", layer);
        const icon = $("img", null, b);
        setSrc(icon, still("icon.png"));
        $("span", null, b, "Wipemark");
      };

      // Underneath: the light frame the wipe uncovers.
      const light = $("div", "hk-light", s.el);
      const lHead = $("div", "hk-head", light, "<div>Wipemark takes them out.</div>");
      const lSub = $("div", "hk-sub", light, "No model. Offline. The same result every time.");
      const lPara = para(light);
      const badgeRow = $("div", "hk-badge", light);
      const badge = $("span", "pill", badgeRow, `<span class="tick">${TICK}</span>Removed: <code>U+200B, U+2060, U+00AD, U+FEFF</code>`);
      lockup(light);

      // On top: the dark frame, and the edge that wipes it away.
      const dark = $("div", "hk-dark", s.el);
      const zoom = $("div", "hk-zoom", dark);
      $("div", "hk-head", zoom, "<div>Your text can carry</div><div>marks you can’t see.</div>");
      const dPara = para(zoom);
      lockup(dark);
      const edge = $("div", "hk-edge", s.el);

      s.update = (t) => {
        zoom.style.transform = `scale(${lerp(1, 1.035, prog(t, T, T + 3.4, E.sine)).toFixed(5)})`;
        const wipe = prog(t, T + 3.3, T + 4.1, E.io);
        dark.style.clipPath = `inset(0 0 0 ${(wipe * 100).toFixed(3)}%)`;
        dark.style.visibility = wipe < 1 ? "visible" : "hidden";
        edge.style.left = px(wipe * 1920);
        edge.style.opacity = wipe > 0 && wipe < 1 ? "1" : "0";
        const close = prog(t, T + 4.15, T + 4.7, E.io);
        MARKS.forEach(([, at], i) => {
          const open = at < 0 ? 1 : prog(t, T + at, T + at + 0.22, E.out);
          const flash = at < 0 ? 0 : 1 - prog(t, T + at + 0.05, T + at + 0.45, E.out);
          const glow = 22 + 10 * Math.sin(6 * t + i * 1.7) + 30 * flash;
          const d = dPara.marks[i];
          d.style.width = px(W_MARK * open);
          d.style.margin = `0 ${px(6 * open)}`;
          const pill = d.querySelector("i");
          pill.style.opacity = clamp(open * 1.4).toFixed(4);
          pill.style.transform = `scale(${(1 + 0.22 * flash).toFixed(4)})`;
          pill.style.boxShadow = `0 0 ${glow.toFixed(1)}px rgba(239, 68, 68, ${(0.55 + 0.3 * flash).toFixed(3)})`;
          // The light paragraph keeps the gaps until the wipe has passed, then
          // closes them, so the words never jump under the edge.
          const l = lPara.marks[i];
          l.style.width = px(W_MARK * open * (1 - close));
          l.style.margin = `0 ${px(6 * open * (1 - close))}`;
          l.querySelector("i").style.opacity = (1 - prog(t, T + 4.15, T + 4.5, E.out)).toFixed(4);
        });
        const sub = prog(t, T + 4.0, T + 4.5, E.out);
        lSub.style.opacity = sub.toFixed(4);
        lSub.style.transform = `translateY(${px(lerp(12, 0, sub))})`;
        lHead.style.opacity = "1";
        const b = prog(t, T + 4.7, T + 5.2, E.out);
        badge.style.opacity = b.toFixed(4);
        badge.style.transform = `translateY(${px(lerp(14, 0, b))})`;
      };
      said(T, T + 3.7, "Your text can carry marks you can’t see.");
      said(T, T + 3.7, "Pasted from an assistant, with U+200B, U+2060, U+00AD and U+FEFF hidden in it.");
      said(T + 3.7, T + 7.2, "Wipemark takes them out. No model. Offline. The same result every time.");
      said(T + 4.7, T + 7.2, "Removed: U+200B, U+2060, U+00AD, U+FEFF");
    }

    // 2. The name.
    {
      const T = 7.05;
      const s = scene(T, T + 3.55, { fin: 0.5, fout: 0.45 });
      const b = brandBlock(s.el, 268, 210, 104);
      const tag = $("div", "brand-line", s.el, "Removes AI provenance marks from your own text and pictures, on your own machine.");
      tag.style.top = "650px";
      s.update = (t) => {
        const i = prog(t, T, T + 0.8, E.out);
        b.icon.style.transform = `scale(${lerp(0.86, 1, i).toFixed(4)})`;
        b.icon.style.opacity = i.toFixed(4);
        const n = prog(t, T + 0.2, T + 0.9, E.out);
        b.name.style.opacity = n.toFixed(4);
        b.name.style.transform = `translateY(${px(lerp(18, 0, n))})`;
        const l = prog(t, T + 0.5, T + 1.2, E.out);
        tag.style.opacity = l.toFixed(4);
        tag.style.transform = `translateY(${px(lerp(18, 0, l))})`;
      };
      said(T + 0.2, T + 3.55, "Wipemark. Removes AI provenance marks from your own text and pictures, on your own machine.");
    }

    // 3. The main window: one list.
    {
      const T = 10.45;
      const s = scene(T, T + 3.8, { fin: 0.5, fout: 0.1 });
      const st = stage(s.el);
      const w = win(st, 2400, 1500, 30, { transparent: true, src: still("main-window.png") });
      s.update = (t) => {
        const rise = prog(t, T, T + 0.8, E.out);
        const drift = prog(t, T, T + 3.7, E.sine);
        place(w, { cx: 1200, cy: lerp(0, 60, drift), s: lerp(0.6, 0.625, drift), ax: 960, ay: lerp(330, 252, rise) });
      };
      cap(T + 0.2, T + 3.6, "Everything lands in one list.", { kicker: "The main window", sub: "Drop, paste or import texts and pictures." });
    }

    // 4. A file dropped on the window, then cleaned (drop-on-main-window.mp4).
    {
      const T = 13.85;
      const s = scene(T, T + 7.45, { fin: 0.3, fout: 0.45 });
      const st = stage(s.el);
      const w = win(st, 2560, 1600, 34);
      // It starts where the still of scene 3 ends (1500 px wide, scrolled
      // 60 of its 2400 px), so the dissolve changes the rows, not the window.
      const start = { cx: 1280, cy: 64, s: 1500 / 2560, ax: 960, ay: 252 };
      const cam = [
        [T, start],
        [T + 1.75, start],
        [T + 2.8, { cx: 974, cy: 340, s: 0.985, ax: 960, ay: 600 }],
        [T + 7.45, { cx: 990, cy: 352, s: 1.0, ax: 960, ay: 600 }],
      ];
      s.update = (t) => {
        // The clip from 3.4 s (the file arriving) to 7.85 s (Cleaned, before
        // the button's tooltip), then held.
        clipFrame(w.img, "drop", Math.min(3.4 + (t - T), 7.85));
        place(w, track(t, cam, E.io));
      };
      cap(T + 0.2, T + 4.1, "Drop a file on the window. Press Clean.", { kicker: "Clean" });
      cap(T + 4.1, T + 7.45, "The result is written beside it.", { kicker: "Clean", sub: "Your file stays as it is." });
    }

    // 5. The report and its three shelves (report.png, the dialog only).
    {
      const T = 21.1;
      const s = scene(T, T + 7.2, { fin: 0.45, fout: 0.45 });
      const st = stage(s.el);
      const w = win(st, 1200, 1120, 22, { src: still("report.png"), imgW: 1296, imgH: 1216, offX: 48, offY: 48 });
      const mark = $("div", "shelf-mark", w.el);
      const shelves = [
        { y: 277, h: 244 },
        { y: 534, h: 270 },
        { y: 820, h: 196 },
      ];
      const cam = [
        [T, { cx: 600, cy: 470, s: 0.78, ax: 960, ay: 640 }],
        [T + 2.5, { cx: 600, cy: 460, s: 0.82, ax: 960, ay: 640 }],
        [T + 3.15, { cx: 600, cy: 690, s: 0.82, ax: 960, ay: 640 }],
        [T + 4.7, { cx: 600, cy: 700, s: 0.82, ax: 960, ay: 640 }],
        [T + 5.35, { cx: 600, cy: 880, s: 0.82, ax: 960, ay: 640 }],
        [T + 7.2, { cx: 600, cy: 892, s: 0.835, ax: 960, ay: 640 }],
      ];
      const markTrack = [
        [T + 2.5, shelves[0]],
        [T + 3.15, shelves[1]],
        [T + 4.7, shelves[1]],
        [T + 5.35, shelves[2]],
      ];
      s.update = (t) => {
        place(w, track(t, cam, E.io));
        const m = track(t, markTrack, E.io);
        Object.assign(mark.style, { left: "10px", width: "1180px", top: px(m.y), height: px(m.h) });
        mark.style.opacity = prog(t, T + 0.45, T + 0.95, E.out).toFixed(4);
      };
      cap(T + 0.2, T + 2.65, "Verifiable: what was removed.", { kicker: "The report", sub: "Counted, so anyone can check it." });
      cap(T + 2.65, T + 4.85, "Best-effort: what was kept, and why.", { kicker: "The report" });
      cap(T + 4.85, T + 7.2, "Not established: what no check settles.", { kicker: "The report", sub: "This shelf is never empty." });
    }

    // 6. Pictures: the provenance blocks leave the file.
    {
      const T = 28.1;
      const s = scene(T, T + 4.0, { fin: 0.45, fout: 0.45 });
      const art = $("div", "pic-art", s.el);
      art.innerHTML =
        '<div class="sun"></div>' +
        '<svg class="hill" viewBox="0 0 400 400" width="400" height="400" style="position:absolute;left:0;top:0">' +
        '<path d="M-10 300 L120 170 L215 262 L262 222 L410 330 L410 410 L-10 410 Z" fill="#1f5ab8" opacity="0.92"/>' +
        '<path d="M-10 340 C80 300 150 318 220 336 C290 354 350 338 410 318 L410 410 L-10 410 Z" fill="#174a99"/>' +
        "</svg>";
      art.style.top = "372px";
      const stack = $("div", "blocks", s.el);
      const label = $("div", null, stack, "picture.png");
      Object.assign(label.style, { position: "absolute", left: "2px", top: "-46px", font: '500 22px "JetBrains Mono"', color: "#525252" });
      const defs = [
        { text: "File header" },
        { text: "Colour profile" },
        { text: "C2PA manifest", ai: true },
        { text: "Camera EXIF" },
        { text: "XMP: made by AI", ai: true },
        { text: "Generator parameters", ai: true },
        { text: "Image data", data: true },
      ];
      const blocks = defs.map((d) => {
        const el = $("div", "block" + (d.ai ? " ai" : "") + (d.data ? " data" : ""), stack, d.text);
        if (d.ai) $("span", "tag", el, "AI");
        if (d.data) d.note = $("code", null, el, "not re-encoded");
        d.el = el;
        return d;
      });
      const STEP = 68;
      s.update = (t) => {
        let yKept = 0;
        let yAll = 0;
        const close = prog(t, T + 2.1, T + 2.85, E.io);
        blocks.forEach((b, i) => {
          const a = prog(t, T + 0.05 + i * 0.09, T + 0.45 + i * 0.09, E.out);
          const gone = b.ai ? prog(t, T + 1.7, T + 2.4, E.in) : 0;
          const pulse = b.ai ? Math.sin(Math.PI * clamp((t - (T + 1.05)) / 0.5)) : 0;
          b.el.style.top = px(lerp(yAll, yKept, close));
          b.el.style.opacity = (a * (1 - gone)).toFixed(4);
          b.el.style.transform = `translateX(${px(lerp(24, 0, a) + 160 * gone)}) scale(${(1 + 0.035 * pulse).toFixed(4)})`;
          yAll += STEP;
          if (!b.ai) yKept += STEP;
          if (b.note) b.note.style.opacity = prog(t, T + 2.75, T + 3.25, E.out).toFixed(4);
        });
        // Keep the stack centred on the picture as it shortens.
        stack.style.top = px(lerp(326, 428, close));
        art.style.opacity = prog(t, T, T + 0.5, E.out).toFixed(4);
      };
      cap(T + 0.2, T + 4.0, "AI provenance metadata comes out.", { kicker: "Pictures", sub: "When only metadata goes, the image data is not re-encoded." });
    }

    // 7. A visible mark, before and after (picture-before-after.png).
    {
      const T = 31.9;
      const s = scene(T, T + 5.2, { fin: 0.45, fout: 0.45 });
      const ba = beforeAfter(s.el, 700, 610, 268);
      s.update = (t) => {
        const ring = prog(t, T + 0.75, T + 1.35, E.out);
        const wipe = prog(t, T + 1.75, T + 3.25, E.io);
        ba.set(ring, wipe, lerp(1, 1.025, prog(t, T, T + 5.2, E.sine)));
      };
      cap(T + 0.2, T + 5.2, "A visible mark comes off only if proved.", {
        kicker: "Pictures",
        sub: "Gemini’s sparkle, and Grok’s mark on pictures from X.",
      });
    }

    // 8. Rewriting: who rewrites (settings-engine.png).
    {
      const T = 36.9;
      const s = scene(T, T + 3.65, { fin: 0.45, fout: 0.15 });
      const st = stage(s.el);
      const w = win(st, 1920, 1360, 28, { transparent: true, src: still("settings-engine.png") });
      const cam = [
        [T, { cx: 960, cy: 520, s: 0.66, ax: 960, ay: 640 }],
        [T + 3.65, { cx: 1130, cy: 420, s: 0.8, ax: 960, ay: 640 }],
      ];
      s.update = (t) => place(w, track(t, cam, E.sine));
      cap(T + 0.2, T + 3.35, "Rewrite, if you want it.", { kicker: "Optional", sub: "With a model on this machine, or an endpoint you choose." });
    }

    // 9. Compare (compare-scroll.mp4).
    {
      const T = 40.1;
      const s = scene(T, T + 5.4, { fin: 0.3, fout: 0.45 });
      const st = stage(s.el);
      const w = win(st, 2560, 1120, 34);
      const cam = [
        [T, { cx: 1280, cy: 560, s: 0.64, ax: 960, ay: 640 }],
        [T + 5.4, { cx: 1280, cy: 540, s: 0.675, ax: 960, ay: 640 }],
      ];
      s.update = (t) => {
        clipFrame(w.img, "compare", 0.8 + (t - T));
        place(w, track(t, cam, E.sine));
      };
      cap(T + 0.15, T + 5.4, "Read the rewrite beside the original.", { kicker: "Compare", sub: "A paragraph that fails a check is never used." });
    }

    // 10. The panel (panel-drop.mp4, then panel-clean.mp4 held on its end).
    {
      const T = 45.3;
      const s = scene(T, T + 6.8, { fin: 0.45, fout: 0.45 });
      const st = stage(s.el);
      const w = win(st, 840, 520, 22);
      s.update = (t) => {
        const cut = T + 2.4;
        if (t < cut) clipFrame(w.img, "pdrop", 1.2 + (t - T));
        else clipFrame(w.img, "pclean", Math.min(1.0 + (t - cut), 3.4));
        const a = prog(t, T, T + 0.6, E.out);
        place(w, { cx: 420, cy: 260, s: lerp(1.27, 1.35, a), ax: 960, ay: 640 });
      };
      cap(T + 0.2, T + 3.2, "Over whatever you are working in.", { kicker: "The panel", sub: "Call it up with a shortcut, drop, press Clean." });
      cap(T + 3.2, T + 6.8, "Everything it takes joins the same list.", { kicker: "The panel" });
    }

    // 11. The command line (cli.png).
    {
      const T = 51.9;
      const s = scene(T, T + 3.8, { fin: 0.45, fout: 0.45 });
      const st = stage(s.el);
      const w = win(st, 2292, 1238, 18, { src: still("cli.png") });
      // From the inspect at the top down to the clean's last lines: "The
      // result is in meeting-notes.cleaned.txt. meeting-notes.txt itself was
      // not changed."
      const cam = [
        [T, { cx: 1146, cy: 380, s: 0.78, ax: 960, ay: 640 }],
        [T + 1.0, { cx: 1146, cy: 400, s: 0.78, ax: 960, ay: 640 }],
        [T + 3.8, { cx: 1146, cy: 690, s: 0.82, ax: 960, ay: 640 }],
      ];
      s.update = (t) => place(w, track(t, cam, E.io));
      cap(T + 0.2, T + 3.8, "The same checks for scripts and CI.", {
        kicker: "The command line",
        sub: "inspect, clean, audit and rewrite, with exit codes for hooks.",
      });
    }

    // 12. Agents over MCP: a call, its answer, the row it leaves in the
    // list; then where it is turned on, the address, a client's snippet and
    // the five tools. Every word is the application's own (the catalogue,
    // src/mcp/mod.rs, src/mcp/protocol.rs); the cards are drawn here.
    {
      const T = 55.5;
      const s = scene(T, T + 8.2, { fin: 0.45, fout: 0.45 });

      // Part one: the call.
      const partA = $("div", null, s.el);
      Object.assign(partA.style, { position: "absolute", inset: "0" });
      const agent = $("div", "mcp-agent", partA, '<div class="bar"><span>your agent</span><b>MCP</b></div>');
      const line = (top, html) => {
        const el = $("div", "ln", agent, html);
        el.style.top = top + "px";
        return el;
      };
      const mkSm = (code) => `<span class="mk sm"><i>${code}</i></span>`;
      const lines = [
        [T + 0.45, line(76, '<span class="muted">call  </span><span class="call">wipemark · clean</span>')],
        [T + 0.75, line(126, `<span class="muted">text  </span>"The rep${mkSm("U+200B")}ort is ready for re${mkSm("U+2060")}view."`)],
        [T + 1.95, line(186, '<span class="muted">back  </span><span class="ok">"The report is ready for review."</span>')],
        [T + 2.2, line(232, '<span class="muted">      report: 2 removed, each with its position</span>')],
      ];
      const wire = $("div", "mcp-wire", partA);
      const wireLabel = $("div", "mcp-wire-label", partA, "127.0.0.1:5056/mcp");
      const dot = $("div", "mcp-dot", partA);
      const row = $(
        "div",
        "mcp-row",
        partA,
        '<div class="head">In Wipemark’s list</div>' +
          '<div class="cells">' +
          '<div class="nm"><b>Text</b><span>From an agent</span><span class="went">Handed back to whoever asked, and kept nowhere.</span></div>' +
          '<div class="kind">Text</div>' +
          '<div class="state">Cleaned</div>' +
          "</div>",
      );

      // Part two: the switch, the address, a snippet, the tools.
      const setup = $("div", "mcp-setup", s.el);
      $("div", "crumb", setup, "Settings › MCP");
      $("div", "serve", setup, "Serve over MCP");
      const toggle = $("div", "toggle", setup, "<i></i>");
      const endpoint = $("div", "endpoint", setup, "Clients connect to <code>http://127.0.0.1:5056/mcp</code>");
      $("div", "rule", setup);
      const snipLabel = $("div", "label", setup, "Connect a client · Claude Code");
      snipLabel.style.left = "40px";
      const snippet = $(
        "pre",
        "snippet",
        setup,
        '{\n  "mcpServers": {\n    "wipemark": {\n      "type": "http",\n      "url": "http://127.0.0.1:5056/mcp"\n    }\n  }\n}',
      );
      const toolsLabel = $("div", "label", setup, "Five tools");
      toolsLabel.style.left = "640px";
      const tools = ["inspect", "clean", "inspect_image", "clean_image", "rewrite"].map((name, i) => {
        const el = $("div", "tool", setup, name);
        el.style.top = 252 + i * 58 + "px";
        return el;
      });

      const WIRE_TOP = 526;
      const WIRE_LEN = 134;
      s.update = (t) => {
        // Part one.
        const outA = prog(t, T + 3.95, T + 4.3, E.in);
        partA.style.opacity = (1 - outA).toFixed(4);
        partA.style.transform = `translateY(${px(-30 * outA)})`;
        const inA = prog(t, T, T + 0.5, E.out);
        agent.style.transform = `translateY(${px(lerp(20, 0, inA))})`;
        for (const [at, el] of lines) {
          const a = prog(t, at, at + 0.3, E.out);
          el.style.opacity = a.toFixed(4);
          el.style.transform = `translateX(${px(lerp(-14, 0, a))})`;
        }
        const w = prog(t, T + 0.7, T + 1.0, E.out);
        wire.style.top = px(WIRE_TOP);
        wire.style.height = px(WIRE_LEN * w);
        wireLabel.style.opacity = w.toFixed(4);
        const down = prog(t, T + 1.0, T + 1.45, E.io);
        const up = prog(t, T + 1.55, T + 1.9, E.io);
        const going = t < T + 1.5;
        dot.style.top = px(WIRE_TOP - 8 + WIRE_LEN * (going ? down : 1 - up));
        dot.style.background = going ? "#3c8bef" : "#2f9e44";
        dot.style.opacity = (t > T + 1.0 && t < T + 1.9) || (down > 0 && down < 1) ? "1" : "0";
        const r = prog(t, T + 1.5, T + 2.0, E.out);
        row.style.opacity = r.toFixed(4);
        row.style.transform = `translateY(${px(lerp(-18, 0, r))})`;

        // Part two.
        const inB = prog(t, T + 4.3, T + 4.75, E.out);
        setup.style.opacity = inB.toFixed(4);
        setup.style.visibility = inB > 0 ? "visible" : "hidden";
        setup.style.transform = `translateY(${px(lerp(24, 0, inB))})`;
        const on = prog(t, T + 4.9, T + 5.15, E.io);
        toggle.style.background = on > 0.5 ? "#3c8bef" : "#d4d4d4";
        toggle.querySelector("i").style.transform = `translateX(${px(28 * on)})`;
        const ep = prog(t, T + 5.2, T + 5.6, E.out);
        endpoint.style.opacity = ep.toFixed(4);
        const sn = prog(t, T + 5.45, T + 5.9, E.out);
        snippet.style.opacity = sn.toFixed(4);
        snipLabel.style.opacity = sn.toFixed(4);
        toolsLabel.style.opacity = prog(t, T + 5.75, T + 6.1, E.out).toFixed(4);
        tools.forEach((el, i) => {
          const a = prog(t, T + 5.85 + i * 0.12, T + 6.2 + i * 0.12, E.out);
          el.style.opacity = a.toFixed(4);
          el.style.transform = `translateX(${px(lerp(16, 0, a))})`;
        });
      };
      cap(T + 0.2, T + 4.2, "Works with your AI agents.", {
        kicker: "MCP",
        sub: "Claude Code, Claude Desktop, n8n or any MCP client.",
      });
      cap(T + 4.2, T + 8.2, "Off until you turn it on.", {
        kicker: "MCP",
        sub: "In Settings › MCP. By default it listens on 127.0.0.1, port 5056.",
      });
      said(T + 0.45, T + 4.2, "Your agent calls wipemark · clean with a text; it gets the text back without the two characters, and a report.");
      said(T + 1.5, T + 4.2, "In Wipemark’s list: Text. From an agent. Handed back to whoever asked, and kept nowhere. Cleaned.");
      said(T + 5.2, T + 8.2, "Serve over MCP. Clients connect to http://127.0.0.1:5056/mcp");
      said(T + 5.75, T + 8.2, "Five tools: inspect, clean, inspect_image, clean_image, rewrite.");
    }

    // 13. Private by default.
    {
      const T = 63.5;
      const s = scene(T, T + 4.4, { fin: 0.45, fout: 0.45 });
      const box = $("div", "points", s.el);
      const lines = [
        [T + 0.5, "Runs on your Mac or your Linux PC."],
        [T + 1.05, "Your files stay here unless you choose an endpoint."],
        [T + 1.6, "No account. No telemetry."],
      ];
      const els = lines.map(([, text], i) => {
        const el = $("div", "point", box, `<span class="tick">${TICK}</span><span>${text}</span>`);
        el.style.top = i * 96 + "px";
        return el;
      });
      box.style.top = "404px";
      // The block is as wide as its longest line, and centred.
      s.layout = () => {
        const widest = Math.max(...els.map((el) => el.scrollWidth));
        box.style.width = widest + "px";
        box.style.marginLeft = -widest / 2 + "px";
      };
      s.update = (t) => {
        els.forEach((el, i) => {
          const a = prog(t, lines[i][0], lines[i][0] + 0.5, E.out);
          el.style.opacity = a.toFixed(4);
          el.style.transform = `translateX(${px(lerp(-22, 0, a))})`;
        });
      };
      cap(T + 0.2, T + 4.4, "Private by default.");
      lines.forEach(([at, text]) => said(at, T + 4.4, text));
    }

    // 14. The end card.
    {
      const T = 67.7;
      const s = scene(T, T + 4.3, { fin: 0.55, fout: 0 });
      const b = brandBlock(s.el, 250, 184, 100);
      const meta = $("div", "end-meta", s.el, 'Version 0.1.0<span class="dot"></span>macOS (Apple Silicon)<span class="dot"></span>Linux (x86_64)');
      meta.style.top = "592px";
      const url = $("div", "end-url", s.el, "github.com/GigLaboCom/wipemark-app");
      url.style.top = "666px";
      const small = $("div", "end-small", s.el, "Free to use · FSL-1.1-ALv2");
      small.style.top = "740px";
      const parts = [
        [b.icon, T],
        [b.name, T + 0.15],
        [meta, T + 0.4],
        [url, T + 0.6],
        [small, T + 0.8],
      ];
      s.update = (t) => {
        for (const [el, at] of parts) {
          const a = prog(t, at, at + 0.6, E.out);
          el.style.opacity = a.toFixed(4);
          el.style.transform = (el === b.icon ? `scale(${lerp(0.9, 1, a).toFixed(4)}) ` : "") + `translateY(${px(lerp(14, 0, a))})`;
        }
      };
      said(T + 0.15, T + 4.3, "Wipemark. Version 0.1.0 · macOS (Apple Silicon) · Linux (x86_64)");
      said(T + 0.6, T + 4.3, "github.com/GigLaboCom/wipemark-app");
      said(T + 0.8, T + 4.3, "Free to use · FSL-1.1-ALv2");
      return T + 4.3;
    }
  }

  // ── The hero loop: 15 seconds, no words, its last frame its first ────
  function buildLoop() {
    function panel(start, end, fin, fout, still0) {
      const s = scene(start, end, { fin, fout });
      const st = stage(s.el, true);
      const w = win(st, 840, 520, 22, { stageTop: 0 });
      s.update = (t) => {
        const lt = t - start;
        if (still0) clipFrame(w.img, "pdrop", 1.2);
        else if (lt < 2.4) clipFrame(w.img, "pdrop", 1.2 + lt);
        else clipFrame(w.img, "pclean", Math.min(1.1 + (lt - 2.4), 3.4));
        place(w, { cx: 420, cy: 260, s: 1.5, ax: 960, ay: 540 });
      };
    }
    panel(0, 5.7, 0, 0.4, false);
    {
      const s = scene(5.3, 10.4, { fin: 0.4, fout: 0.4 });
      const st = stage(s.el, true);
      const w = win(st, 2560, 1600, 34, { stageTop: 0 });
      s.update = (t) => {
        clipFrame(w.img, "drop", Math.min(4.9 + (t - 5.3), 7.85));
        const d = prog(t, 5.3, 10.4, E.sine);
        place(w, { cx: lerp(974, 990, d), cy: lerp(300, 312, d), s: lerp(0.985, 1.0, d), ax: 960, ay: 470 });
      };
    }
    {
      const s = scene(10.0, 15.0, { fin: 0.4, fout: 0.55 });
      const ba = beforeAfter(s.el, 800, 560, 140);
      s.update = (t) => {
        const ring = prog(t, 10.6, 11.2, E.out);
        const wipe = prog(t, 11.5, 12.9, E.io);
        ba.set(ring, wipe, lerp(1, 1.02, prog(t, 10.0, 15.0, E.sine)));
      };
    }
    // The first frame again, dissolving in, so the loop has no seam.
    panel(14.45, 15.0 + 1, 0.55, 0, true);
    return 15.0;
  }

  // ── Driving it ──────────────────────────────────────────────────────
  const builders = { main: buildMain, loop: buildLoop };
  if (!builders[CUT]) throw new Error("unknown cut " + CUT);
  const duration = builders[CUT]();

  function seek(t) {
    for (const s of scenes) {
      let o = 0;
      if (t >= s.start && t < s.end) {
        const fin = s.fin > 0 ? prog(t, s.start, s.start + s.fin, E.sine) : 1;
        const fout = s.fout > 0 ? 1 - prog(t, s.end - s.fout, s.end, E.sine) : 1;
        o = fin * fout;
      }
      s.el.style.visibility = o > 0.0005 ? "visible" : "hidden";
      s.el.style.opacity = o.toFixed(4);
      if (o > 0.0005) s.update(t);
    }
    updateCaptions(t);
    return Promise.all(pending.splice(0));
  }

  window.WM = {
    fps: FPS,
    duration,
    async ready() {
      await document.fonts.load('400 40px "Inter"');
      await document.fonts.load('500 40px "Inter"');
      await document.fonts.load('600 40px "Inter"');
      await document.fonts.load('700 40px "Inter"');
      await document.fonts.load('400 20px "JetBrains Mono"');
      await document.fonts.load('600 20px "JetBrains Mono"');
      await document.fonts.ready;
      for (const s of scenes) if (s.layout) s.layout();
      await Promise.all(pending.splice(0));
    },
    seek,
    cues() {
      return cues
        .map((c) => ({
          start: c.start,
          end: c.end,
          text: c.band ? [c.head, c.sub].filter(Boolean).join("\n") : c.text,
          kicker: c.band ? c.kicker : "",
        }))
        .sort((a, b) => a.start - b.start || a.end - b.end);
    },
    // The words that must survive a square crop of the frame's centre:
    // every one lies between x 430 and 1490, the centre 1080 px less 10 px
    // each side. Measured where each rests, before anything moves it.
    tooWide() {
      const out = [];
      const sel = [
        ".cap .kicker", ".cap .head", ".cap .sub",
        ".hk-head div", ".hk-sub", ".hk-label", ".hk-line", ".hk-brand", ".pill",
        ".brand-name", ".brand-line", ".point",
        ".mcp-agent .ln", ".mcp-agent .bar", ".mcp-wire-label", ".mcp-row .head", ".mcp-row .cells",
        ".mcp-setup .crumb", ".mcp-setup .serve", ".mcp-setup .toggle", ".mcp-setup .endpoint",
        ".mcp-setup .label", ".mcp-setup .snippet", ".mcp-setup .tool",
        ".end-meta", ".end-url", ".end-small",
      ].join(", ");
      for (const el of document.querySelectorAll(sel)) {
        const r = document.createRange();
        r.selectNodeContents(el);
        const box = r.getBoundingClientRect();
        if (box.width > 0 && (box.left < 430 || box.right > 1490)) {
          out.push({ text: el.textContent.trim().slice(0, 60), left: Math.round(box.left), right: Math.round(box.right) });
        }
      }
      return out;
    },
    // Every character any frame can show, so render.mjs can refuse a glyph
    // the bundled fonts do not have (Chrome would borrow a system font).
    text() {
      return document.getElementById("frame").textContent + cues.map((c) => (c.band ? c.kicker + c.head + c.sub : c.text)).join("");
    },
  };
})();
