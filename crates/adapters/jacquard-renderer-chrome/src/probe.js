// Jacquard layout probe. Measures, never judges: jacquard-core decides.
// Executed either from the Chrome outer wrapper with frozen host bindings
// against a candidate iframe's Document, or directly over Playwright's CDP
// evaluate channel returning the measurement object by value.
(function (targetDoc, targetWin, hostApi) {
  "use strict";
  const doc = targetDoc || document;
  const win = targetWin || window;
  const H = hostApi || {
    ArrayFrom: Array.from.bind(Array),
    MapCtor: Map,
    Int32ArrayCtor: Int32Array,
    MathFloor: Math.floor,
    MathMin: Math.min,
    MathMax: Math.max,
    MathRound: Math.round,
    MathPow: Math.pow,
    parseFloat: Number.parseFloat,
    parseInt: Number.parseInt,
    NumberFn: Number,
    BooleanFn: Boolean,
    getComputedStyle: window.getComputedStyle.bind(window),
    getPropVal: (st, k) => st.getPropertyValue(k),
    getBCR: (el) => el.getBoundingClientRect(),
    getClientRects: (el) => el.getClientRects(),
    matches: (el, sel) => el.matches(sel),
    qsa: (el, sel) => el.querySelectorAll(sel),
    scrollWidth: (el) => el.scrollWidth,
    clientWidth: (el) => el.clientWidth,
    scrollHeight: (el) => el.scrollHeight,
    clientHeight: (el) => el.clientHeight,
    tagName: (el) => el.tagName,
    id: (el) => el.id,
    classList: (el) => el.classList,
    childNodes: (n) => n.childNodes,
    nodeType: (n) => n.nodeType,
    textContent: (n) => n.textContent,
    parentElement: (n) => n.parentElement,
    contains: (a, b) => a.contains(b),
    docBody: (d) => d.body,
    docEl: (d) => d.documentElement,
    createRange: (d) => d.createRange(),
    selectNodeContents: (r, n) => r.selectNodeContents(n),
    rangeRects: (r) => r.getClientRects(),
    innerWidth: (w) => w.innerWidth,
    innerHeight: (w) => w.innerHeight,
    scrollX: (w) => w.scrollX,
    scrollY: (w) => w.scrollY,
    makeCanvasCtx: () => {
      const cv = document.createElement("canvas");
      cv.width = 1;
      cv.height = 1;
      return cv.getContext("2d", { willReadFrequently: true });
    },
    ctxClear: (c, x, y, w, h) => c.clearRect(x, y, w, h),
    ctxSetFill: (c, v) => { c.fillStyle = v; },
    ctxFillRect: (c, x, y, w, h) => c.fillRect(x, y, w, h),
    ctxImageData: (c, x, y, w, h) => c.getImageData(x, y, w, h).data,
  };

  function describe(el) {
    let s = H.tagName(el).toLowerCase();
    const elId = H.id(el);
    if (elId) s += "#" + elId;
    const cl = H.classList(el);
    if (cl && cl.length) s += "." + H.ArrayFrom(cl).join(".");
    const text = (H.textContent(el) || "").trim().replace(/\s+/g, " ").slice(0, 32);
    return text ? s + ' "' + text + '"' : s;
  }

  // Seen by a sighted visitor: rendered, not visually hidden (a box of at
  // most 1px, or clip-path inset(50%), on it or an ancestor: the usual
  // screen-reader-only pattern), and not entirely above or left of the
  // document, where no scrolling reaches (a skip link waiting for focus).
  function visible(el) {
    const st = H.getComputedStyle(el);
    if (
      H.getPropVal(st, "visibility") === "hidden" ||
      H.getPropVal(st, "display") === "none" ||
      H.getClientRects(el).length === 0
    ) {
      return false;
    }
    const r = H.getBCR(el);
    const sx = H.scrollX(win);
    const sy = H.scrollY(win);
    if (r.bottom + sy <= 0 || r.right + sx <= 0) return false;
    for (let n = el; n && H.nodeType(n) === 1; n = H.parentElement(n)) {
      const s = H.getComputedStyle(n);
      const b = H.getBCR(n);
      if (
        H.getPropVal(s, "clip-path") === "inset(50%)" ||
        (b.width <= 1 && b.height <= 1 && H.getPropVal(s, "overflow") !== "visible")
      ) {
        return false;
      }
    }
    return true;
  }

  function ownText(el) {
    return H.ArrayFrom(H.childNodes(el)).some(
      (n) => H.nodeType(n) === 3 && H.textContent(n).trim()
    );
  }

  function textRect(el) {
    let box = null;
    for (const n of H.ArrayFrom(H.childNodes(el))) {
      if (H.nodeType(n) !== 3 || !H.textContent(n).trim()) continue;
      const range = H.createRange(doc);
      H.selectNodeContents(range, n);
      for (const r of H.ArrayFrom(H.rangeRects(range))) {
        if (r.width < 1 || r.height < 1) continue;
        box = box
          ? {
              l: H.MathMin(box.l, r.left),
              t: H.MathMin(box.t, r.top),
              r: H.MathMax(box.r, r.right),
              b: H.MathMax(box.b, r.bottom),
            }
          : { l: r.left, t: r.top, r: r.right, b: r.bottom };
      }
    }
    return box;
  }

  let colorCtx = null;
  function getColorCtx() {
    if (!colorCtx) colorCtx = H.makeCanvasCtx();
    return colorCtx;
  }

  function parse(c) {
    if (!c || c === "transparent") return { r: 0, g: 0, b: 0, a: 0 };
    const m = c.match(/^rgba?\(([^)]+)\)$/);
    if (m) {
      const p = m[1].split(/[ ,/]+/).filter(H.BooleanFn).map(H.NumberFn);
      return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 };
    }
    const ctx = getColorCtx();
    if (!ctx) return null;
    H.ctxClear(ctx, 0, 0, 1, 1);
    H.ctxSetFill(ctx, "rgba(0,0,0,0)");
    H.ctxSetFill(ctx, c);
    H.ctxFillRect(ctx, 0, 0, 1, 1);
    const d = H.ctxImageData(ctx, 0, 0, 1, 1);
    return { r: d[0], g: d[1], b: d[2], a: d[3] / 255 };
  }

  function lum(c) {
    const f = (v) => {
      v /= 255;
      return v <= 0.04045 ? v / 12.92 : H.MathPow((v + 0.055) / 1.055, 2.4);
    };
    return 0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b);
  }

  function over(top, under) {
    const a = top.a;
    return {
      r: top.r * a + under.r * (1 - a),
      g: top.g * a + under.g * (1 - a),
      b: top.b * a + under.b * (1 - a),
      a: 1,
    };
  }

  function background(el) {
    const layers = [];
    for (let n = el; n && H.nodeType(n) === 1; n = H.parentElement(n)) {
      const c = parse(H.getPropVal(H.getComputedStyle(n), "background-color"));
      if (c && c.a > 0) {
        layers.push(c);
        if (c.a >= 1) break;
      }
    }
    let bg = { r: 255, g: 255, b: 255, a: 1 };
    for (let i = layers.length - 1; i >= 0; i--) bg = over(layers[i], bg);
    return bg;
  }

  const rootEl = H.docEl(doc);
  const bodyEl = H.docBody(doc);
  const out = {
    viewport_width: H.innerWidth(win),
    viewport_height: H.innerHeight(win),
    document_width: rootEl ? H.scrollWidth(rootEl) : 0,
    clipped: [],
    overlaps: [],
    contrast: [],
    targets: [],
  };
  if (!bodyEl) return out;

  const all = H.ArrayFrom(H.qsa(bodyEl, "*")).filter((el) => {
    const tn = H.tagName(el);
    return tn !== "SCRIPT" && tn !== "STYLE" && visible(el);
  });
  const texts = all.filter(ownText);
  for (const el of texts) {
    const st = H.getComputedStyle(el);
    const hides = (v) => v === "hidden" || v === "clip" || v === "scroll" || v === "auto";
    if (
      (hides(H.getPropVal(st, "overflow-x")) && H.scrollWidth(el) > H.clientWidth(el) + 1) ||
      (hides(H.getPropVal(st, "overflow-y")) && H.scrollHeight(el) > H.clientHeight(el) + 1)
    ) {
      out.clipped.push(describe(el));
    }
    const fg = parse(H.getPropVal(st, "color"));
    // Fully transparent text is not seen, so it has no contrast to judge.
    if (fg && fg.a > 0 && H.parseFloat(H.getPropVal(st, "opacity")) > 0) {
      const bg = background(el);
      const top = over(fg, bg);
      const sorted = [lum(top), lum(bg)].sort((a, b) => b - a);
      const hi = sorted[0], lo = sorted[1];
      const size = H.parseFloat(H.getPropVal(st, "font-size"));
      const weight = H.parseInt(H.getPropVal(st, "font-weight"), 10) || 400;
      out.contrast.push({
        element: describe(el),
        ratio: H.MathRound(((hi + 0.05) / (lo + 0.05)) * 100) / 100,
        large: size >= 24 || (size >= 18.66 && weight >= 700),
      });
    }
  }

  const boxes = texts.map((el) => ({ el, box: textRect(el) })).filter((x) => x.box);
  const CELL = 128;
  const grid = new H.MapCtor();
  const cellsOf = (b) => [
    H.MathFloor(b.l / CELL),
    H.MathFloor(b.r / CELL),
    H.MathFloor(b.t / CELL),
    H.MathFloor(b.b / CELL),
  ];
  for (let i = 0; i < boxes.length; i++) {
    const cs = cellsOf(boxes[i].box);
    for (let gx = cs[0]; gx <= cs[1]; gx++) {
      for (let gy = cs[2]; gy <= cs[3]; gy++) {
        const k = gx + "," + gy;
        let list = grid.get(k);
        if (!list) { list = []; grid.set(k, list); }
        list.push(i);
      }
    }
  }
  const stamp = new H.Int32ArrayCtor(boxes.length).fill(-1);
  for (let i = 0; i < boxes.length; i++) {
    const a = boxes[i];
    const cs = cellsOf(a.box);
    for (let gx = cs[0]; gx <= cs[1]; gx++) {
      for (let gy = cs[2]; gy <= cs[3]; gy++) {
        const list = grid.get(gx + "," + gy);
        if (!list) continue;
        for (let m = 0; m < list.length; m++) {
          const j = list[m];
          if (j <= i || stamp[j] === i) continue;
          stamp[j] = i;
          const b = boxes[j];
          if (H.contains(a.el, b.el) || H.contains(b.el, a.el)) continue;
          const w = H.MathMin(a.box.r, b.box.r) - H.MathMax(a.box.l, b.box.l);
          const h = H.MathMin(a.box.b, b.box.b) - H.MathMax(a.box.t, b.box.t);
          if (w > 1 && h > 1) out.overlaps.push([describe(a.el), describe(b.el)]);
        }
      }
    }
  }

  const interactive = all.filter((el) =>
    H.matches(
      el,
      "a[href], button, input:not([type=hidden]), select, textarea, [role=button], [tabindex]:not([tabindex='-1'])"
    )
  );
  for (const el of interactive) {
    const r = H.getBCR(el);
    const st = H.getComputedStyle(el);
    const parent = H.parentElement(el);
    const inline =
      H.getPropVal(st, "display") === "inline" &&
      !!parent &&
      H.ArrayFrom(H.childNodes(parent)).some(
        (n) => H.nodeType(n) === 3 && H.textContent(n).trim()
      );
    out.targets.push({
      element: describe(el),
      width: H.MathRound(r.width * 10) / 10,
      height: H.MathRound(r.height * 10) / 10,
      inline,
    });
  }
  return out;
})
