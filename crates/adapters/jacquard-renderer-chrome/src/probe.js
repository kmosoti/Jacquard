// Jacquard layout probe. Measures, never judges: jacquard-core decides.
// Posts its report to the wrapper frame, which writes it into
// <script id="jacquard-probe">; at top level it writes the element itself.
(function () {
  "use strict";
  function describe(el) {
    let s = el.tagName.toLowerCase();
    if (el.id) s += "#" + el.id;
    if (el.classList && el.classList.length) s += "." + Array.from(el.classList).join(".");
    const text = (el.textContent || "").trim().replace(/\s+/g, " ").slice(0, 32);
    return text ? s + ' "' + text + '"' : s;
  }
  // Seen by a sighted visitor: rendered, not visually hidden (a box of at
  // most 1px, or clip-path inset(50%), on it or an ancestor: the usual
  // screen-reader-only pattern), and not entirely above or left of the
  // document, where no scrolling reaches (a skip link waiting for focus).
  function visible(el) {
    const st = getComputedStyle(el);
    if (st.visibility === "hidden" || st.display === "none" || el.getClientRects().length === 0) return false;
    const r = el.getBoundingClientRect();
    if (r.bottom + scrollY <= 0 || r.right + scrollX <= 0) return false;
    for (let n = el; n && n.nodeType === 1; n = n.parentElement) {
      const s = getComputedStyle(n);
      const b = n.getBoundingClientRect();
      if (s.clipPath === "inset(50%)" || (b.width <= 1 && b.height <= 1 && s.overflow !== "visible")) return false;
    }
    return true;
  }
  function ownText(el) {
    return Array.from(el.childNodes).some((n) => n.nodeType === 3 && n.textContent.trim());
  }
  function textRect(el) {
    let box = null;
    for (const n of el.childNodes) {
      if (n.nodeType !== 3 || !n.textContent.trim()) continue;
      const range = document.createRange();
      range.selectNodeContents(n);
      for (const r of range.getClientRects()) {
        if (r.width < 1 || r.height < 1) continue;
        box = box
          ? { l: Math.min(box.l, r.left), t: Math.min(box.t, r.top), r: Math.max(box.r, r.right), b: Math.max(box.b, r.bottom) }
          : { l: r.left, t: r.top, r: r.right, b: r.bottom };
      }
    }
    return box;
  }
  let colorCtx = null;
  function getColorCtx() {
    if (!colorCtx) {
      const cv = document.createElement("canvas");
      cv.width = 1;
      cv.height = 1;
      colorCtx = cv.getContext("2d", { willReadFrequently: true });
    }
    return colorCtx;
  }
  function parse(c) {
    if (!c || c === "transparent") return { r: 0, g: 0, b: 0, a: 0 };
    const m = c.match(/^rgba?\(([^)]+)\)$/);
    if (m) {
      const p = m[1].split(/[ ,/]+/).filter(Boolean).map(Number);
      return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 };
    }
    const ctx = getColorCtx();
    if (!ctx) return null;
    ctx.clearRect(0, 0, 1, 1);
    ctx.fillStyle = "rgba(0,0,0,0)";
    ctx.fillStyle = c;
    ctx.fillRect(0, 0, 1, 1);
    const d = ctx.getImageData(0, 0, 1, 1).data;
    return { r: d[0], g: d[1], b: d[2], a: d[3] / 255 };
  }
  function lum(c) {
    const f = (v) => { v /= 255; return v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); };
    return 0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b);
  }
  function over(top, under) {
    const a = top.a;
    return { r: top.r * a + under.r * (1 - a), g: top.g * a + under.g * (1 - a), b: top.b * a + under.b * (1 - a), a: 1 };
  }
  function background(el) {
    const layers = [];
    for (let n = el; n && n.nodeType === 1; n = n.parentElement) {
      const c = parse(getComputedStyle(n).backgroundColor);
      if (c && c.a > 0) { layers.push(c); if (c.a >= 1) break; }
    }
    let bg = { r: 255, g: 255, b: 255, a: 1 };
    for (let i = layers.length - 1; i >= 0; i--) bg = over(layers[i], bg);
    return bg;
  }
  function run() {
    const out = { viewport_width: innerWidth, viewport_height: innerHeight, document_width: document.documentElement.scrollWidth, clipped: [], overlaps: [], contrast: [], targets: [] };
    const all = Array.from(document.body.querySelectorAll("*")).filter((el) => el.tagName !== "SCRIPT" && el.tagName !== "STYLE" && visible(el));
    const texts = all.filter(ownText);
    for (const el of texts) {
      const st = getComputedStyle(el);
      const hides = (v) => v === "hidden" || v === "clip" || v === "scroll" || v === "auto";
      if ((hides(st.overflowX) && el.scrollWidth > el.clientWidth + 1) || (hides(st.overflowY) && el.scrollHeight > el.clientHeight + 1)) {
        out.clipped.push(describe(el));
      }
      const fg = parse(st.color);
      // Fully transparent text is not seen, so it has no contrast to judge.
      if (fg && fg.a > 0 && parseFloat(st.opacity) > 0) {
        const bg = background(el);
        const top = over(fg, bg);
        const [hi, lo] = [lum(top), lum(bg)].sort((a, b) => b - a);
        const size = parseFloat(st.fontSize), weight = parseInt(st.fontWeight, 10) || 400;
        out.contrast.push({ element: describe(el), ratio: Math.round(((hi + 0.05) / (lo + 0.05)) * 100) / 100, large: size >= 24 || (size >= 18.66 && weight >= 700) });
      }
    }
    const boxes = texts.map((el) => ({ el, box: textRect(el) })).filter((x) => x.box);
    const CELL = 128;
    const grid = new Map();
    const cellsOf = (b) => [
      Math.floor(b.l / CELL),
      Math.floor(b.r / CELL),
      Math.floor(b.t / CELL),
      Math.floor(b.b / CELL),
    ];
    for (let i = 0; i < boxes.length; i++) {
      const [x0, x1, y0, y1] = cellsOf(boxes[i].box);
      for (let gx = x0; gx <= x1; gx++) {
        for (let gy = y0; gy <= y1; gy++) {
          const k = gx + "," + gy;
          let list = grid.get(k);
          if (!list) { list = []; grid.set(k, list); }
          list.push(i);
        }
      }
    }
    const stamp = new Int32Array(boxes.length).fill(-1);
    for (let i = 0; i < boxes.length; i++) {
      const a = boxes[i];
      const [x0, x1, y0, y1] = cellsOf(a.box);
      for (let gx = x0; gx <= x1; gx++) {
        for (let gy = y0; gy <= y1; gy++) {
          const list = grid.get(gx + "," + gy);
          if (!list) continue;
          for (let m = 0; m < list.length; m++) {
            const j = list[m];
            if (j <= i || stamp[j] === i) continue;
            stamp[j] = i;
            const b = boxes[j];
            if (a.el.contains(b.el) || b.el.contains(a.el)) continue;
            const w = Math.min(a.box.r, b.box.r) - Math.max(a.box.l, b.box.l);
            const h = Math.min(a.box.b, b.box.b) - Math.max(a.box.t, b.box.t);
            if (w > 1 && h > 1) out.overlaps.push([describe(a.el), describe(b.el)]);
          }
        }
      }
    }
    const interactive = all.filter((el) => el.matches("a[href], button, input:not([type=hidden]), select, textarea, [role=button], [tabindex]:not([tabindex='-1'])"));
    for (const el of interactive) {
      const r = el.getBoundingClientRect();
      const st = getComputedStyle(el);
      const parent = el.parentElement;
      const inline = st.display === "inline" && !!parent && Array.from(parent.childNodes).some((n) => n.nodeType === 3 && n.textContent.trim());
      out.targets.push({ element: describe(el), width: Math.round(r.width * 10) / 10, height: Math.round(r.height * 10) / 10, inline });
    }
    const json = JSON.stringify(out).replace(/</g, "\\u003c");
    if (window.parent !== window) {
      window.parent.postMessage("jacquard-probe:" + json, "*");
    } else {
      const tag = document.createElement("script");
      tag.type = "application/json";
      tag.id = "jacquard-probe";
      tag.textContent = json;
      document.head.appendChild(tag);
    }
  }
  // Not requestAnimationFrame: it never fires in a frame under --dump-dom.
  const start = () => (document.fonts ? document.fonts.ready : Promise.resolve()).then(() => setTimeout(run, 0));
  if (document.readyState === "complete") start(); else addEventListener("load", start);
})();
