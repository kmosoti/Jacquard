(function () {
  "use strict";
  const cs = document.currentScript;
  if (cs && cs.parentNode) cs.parentNode.removeChild(cs);

  const PROBE_ID = __JACQUARD_PROBE_ID_JSON__;
  const ENTRY_URL = __JACQUARD_ENTRY_URL_JSON__;
  const KEY = __JACQUARD_MAC_KEY_JSON__;
  const IPAD = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
  const OPAD = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0];
  for (let i = 0; i < 64; i++) {
    const kb = i < KEY.length ? (KEY[i] & 255) : 0;
    IPAD[i] = kb ^ 0x36;
    OPAD[i] = kb ^ 0x5c;
    if (i < KEY.length) KEY[i] = 0;
  }

  const freeze = Object.freeze;
  const gd = Object.getOwnPropertyDescriptor;
  const getProto = Object.getPrototypeOf;
  const call = Function.prototype.call;
  const getFn = (p, k) => {
    const d = gd(p, k);
    return d && d.get ? (o) => call.call(d.get, o) : null;
  };
  const setFn = (p, k) => {
    const d = gd(p, k);
    return d && d.set ? (o, v) => call.call(d.set, o, v) : null;
  };

  const HEX = freeze(["0","1","2","3","4","5","6","7","8","9","a","b","c","d","e","f"]);
  const SHA_K = freeze([
    0x428a2f98|0, 0x71374491|0, 0xb5c0fbcf|0, 0xe9b5dba5|0, 0x3956c25b|0, 0x59f111f1|0, 0x923f82a4|0, 0xab1c5ed5|0,
    0xd807aa98|0, 0x12835b01|0, 0x243185be|0, 0x550c7dc3|0, 0x72be5d74|0, 0x80deb1fe|0, 0x9bdc06a7|0, 0xc19bf174|0,
    0xe49b69c1|0, 0xefbe4786|0, 0x0fc19dc6|0, 0x240ca1cc|0, 0x2de92c6f|0, 0x4a7484aa|0, 0x5cb0a9dc|0, 0x76f988da|0,
    0x983e5152|0, 0xa831c66d|0, 0xb00327c8|0, 0xbf597fc7|0, 0xc6e00bf3|0, 0xd5a79147|0, 0x06ca6351|0, 0x14292967|0,
    0x27b70a85|0, 0x2e1b2138|0, 0x4d2c6dfc|0, 0x53380d13|0, 0x650a7354|0, 0x766a0abb|0, 0x81c2c92e|0, 0x92722c85|0,
    0xa2bfe8a1|0, 0xa81a664b|0, 0xc24b8b70|0, 0xc76c51a3|0, 0xd192e819|0, 0xd6990624|0, 0xf40e3585|0, 0x106aa070|0,
    0x19a4c116|0, 0x1e376c08|0, 0x2748774c|0, 0x34b0bcb5|0, 0x391c0cb3|0, 0x4ed8aa4a|0, 0x5b9cca4f|0, 0x682e6ff3|0,
    0x748f82ee|0, 0x78a5636f|0, 0x84c87814|0, 0x8cc70208|0, 0x90befffa|0, 0xa4506ceb|0, 0xbef9a3f7|0, 0xc67178f2|0
  ]);
  freeze(IPAD);
  freeze(OPAD);

  const iwGet = getFn(window, "innerWidth") || ((w) => w.innerWidth);
  const ihGet = getFn(window, "innerHeight") || ((w) => w.innerHeight);
  const sxGet = getFn(window, "scrollX") || ((w) => w.scrollX);
  const syGet = getFn(window, "scrollY") || ((w) => w.scrollY);
  const gcs = window.getComputedStyle.bind(window);
  const gpv = CSSStyleDeclaration.prototype.getPropertyValue;
  const gbcr = Element.prototype.getBoundingClientRect;
  const gcr = Element.prototype.getClientRects;
  const em = Element.prototype.matches;
  const eqsa = Element.prototype.querySelectorAll;
  const dqsa = Document.prototype.querySelectorAll;
  const dCreate = Document.prototype.createElement;
  const dRange = Document.prototype.createRange;
  const eSetAttr = Element.prototype.setAttribute;
  const eTagName = getFn(Element.prototype, "tagName");
  const nFirstChild = getFn(Node.prototype, "firstChild");
  const nNextSibling = getFn(Node.prototype, "nextSibling");
  const nParentNode = getFn(Node.prototype, "parentNode");
  const nRemove = Node.prototype.removeChild;
  const nAppend = Node.prototype.appendChild;
  const nContains = Node.prototype.contains;
  const nTypeGet = getFn(Node.prototype, "nodeType");
  const nTextSet = setFn(Node.prototype, "textContent");
  const nTextGet = getFn(Node.prototype, "textContent");
  const eAdd = EventTarget.prototype.addEventListener;
  const fSrcSet = setFn(HTMLIFrameElement.prototype, "src");
  const fDoc = getFn(HTMLIFrameElement.prototype, "contentDocument");
  const fWin = getFn(HTMLIFrameElement.prototype, "contentWindow");
  const dBody = getFn(Document.prototype, "body");
  const dEl = getFn(Document.prototype, "documentElement");
  const dHead = getFn(Document.prototype, "head");
  const dFonts = getFn(Document.prototype, "fonts");
  const fFontsReady = typeof FontFaceSet !== "undefined" ? getFn(FontFaceSet.prototype, "ready") : null;
  const cCtx = HTMLCanvasElement.prototype.getContext;
  const cvWidthSet = setFn(HTMLCanvasElement.prototype, "width");
  const cvHeightSet = setFn(HTMLCanvasElement.prototype, "height");
  const cClear = CanvasRenderingContext2D.prototype.clearRect;
  const cFillSet = setFn(CanvasRenderingContext2D.prototype, "fillStyle");
  const cFillRect = CanvasRenderingContext2D.prototype.fillRect;
  const cImgData = CanvasRenderingContext2D.prototype.getImageData;
  const imgDataGet = getFn(ImageData.prototype, "data");
  const rSelect = Range.prototype.selectNodeContents;
  const rRects = Range.prototype.getClientRects;
  const pRes = Promise.resolve.bind(Promise);
  const pThen = Promise.prototype.then;
  const setTO = window.setTimeout.bind(window);
  const jsonStr = JSON.stringify;
  const arrJoin = Array.prototype.join;
  const strCodeAt = String.prototype.charCodeAt;
  const mFloor = Math.floor;
  const NumberFn = Number;
  const BooleanFn = Boolean;
  const headEl = dHead(document);
  const docEl = dEl(document);

  const safeFreeze = (x) => { try { if (x) freeze(x); } catch (_) {} };
  safeFreeze(Object); safeFreeze(Object.prototype);
  safeFreeze(Array); safeFreeze(Array.prototype);
  safeFreeze(String); safeFreeze(String.prototype);
  safeFreeze(Number); safeFreeze(Number.prototype);
  safeFreeze(Boolean); safeFreeze(Boolean.prototype);
  safeFreeze(Function); safeFreeze(Function.prototype);
  safeFreeze(Map); safeFreeze(Map.prototype);
  safeFreeze(Int32Array); safeFreeze(Int32Array.prototype);
  safeFreeze(Uint32Array); safeFreeze(Uint32Array.prototype);
  safeFreeze(Uint8Array); safeFreeze(Uint8Array.prototype);
  safeFreeze(Uint8ClampedArray.prototype);
  safeFreeze(getProto(Int32Array.prototype));
  safeFreeze(ArrayBuffer); safeFreeze(ArrayBuffer.prototype);
  safeFreeze(Promise); safeFreeze(Promise.prototype);
  safeFreeze(RegExp); safeFreeze(RegExp.prototype);
  safeFreeze(JSON); safeFreeze(Math);
  safeFreeze(Node.prototype); safeFreeze(Element.prototype);
  safeFreeze(Document.prototype); safeFreeze(HTMLIFrameElement.prototype);
  safeFreeze(HTMLCanvasElement.prototype); safeFreeze(CSSStyleDeclaration.prototype);
  safeFreeze(Range.prototype); safeFreeze(ImageData.prototype);
  if (typeof FontFaceSet !== "undefined") safeFreeze(FontFaceSet.prototype);
  if (typeof DOMTokenList !== "undefined") safeFreeze(DOMTokenList.prototype);

  const hostApi = freeze({
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
    NumberFn: NumberFn,
    BooleanFn: BooleanFn,
    getComputedStyle: (el) => gcs(el),
    getPropVal: (st, k) => call.call(gpv, st, k),
    getBCR: (el) => call.call(gbcr, el),
    getClientRects: (el) => call.call(gcr, el),
    matches: (el, s) => call.call(em, el, s),
    qsa: (el, s) => call.call(eqsa, el, s),
    scrollWidth: getFn(Element.prototype, "scrollWidth"),
    clientWidth: getFn(Element.prototype, "clientWidth"),
    scrollHeight: getFn(Element.prototype, "scrollHeight"),
    clientHeight: getFn(Element.prototype, "clientHeight"),
    tagName: eTagName,
    id: getFn(Element.prototype, "id"),
    classList: getFn(Element.prototype, "classList"),
    childNodes: getFn(Node.prototype, "childNodes"),
    nodeType: nTypeGet,
    textContent: nTextGet,
    parentElement: getFn(Node.prototype, "parentElement"),
    contains: (a, b) => call.call(nContains, a, b),
    docBody: dBody,
    docEl: dEl,
    createRange: (d) => call.call(dRange, d),
    selectNodeContents: (r, n) => call.call(rSelect, r, n),
    rangeRects: (r) => call.call(rRects, r),
    innerWidth: iwGet,
    innerHeight: ihGet,
    scrollX: sxGet,
    scrollY: syGet,
    makeCanvasCtx: () => {
      const cv = call.call(dCreate, document, "canvas");
      cvWidthSet(cv, 1);
      cvHeightSet(cv, 1);
      return call.call(cCtx, cv, "2d", { willReadFrequently: true });
    },
    ctxClear: (c, x, y, w, h) => call.call(cClear, c, x, y, w, h),
    ctxSetFill: (c, v) => cFillSet(c, v),
    ctxFillRect: (c, x, y, w, h) => call.call(cFillRect, c, x, y, w, h),
    ctxImageData: (c, x, y, w, h) => imgDataGet(call.call(cImgData, c, x, y, w, h)),
  });

  function asciiEscapeJson(raw) {
    const out = [];
    let oi = 0;
    for (let i = 0; i < raw.length; i++) {
      const c = call.call(strCodeAt, raw, i);
      if (c === 0x3c || c === 0x3e || c === 0x26 || c >= 0x7f) {
        out[oi++] = "\\u" + HEX[(c >>> 12) & 15] + HEX[(c >>> 8) & 15] + HEX[(c >>> 4) & 15] + HEX[c & 15];
      } else {
        out[oi++] = raw[i];
      }
    }
    return call.call(arrJoin, out, "");
  }

  function rotr(x, n) {
    return (x >>> n) | (x << (32 - n));
  }

  function sha256Bytes(prefix, data) {
    const total = prefix.length + data.length;
    const paddedLen = ((total + 9 + 63) >>> 6) << 6;
    const buf = [];
    for (let i = 0; i < paddedLen; i++) buf[i] = 0;
    for (let i = 0; i < prefix.length; i++) buf[i] = prefix[i] & 255;
    for (let i = 0; i < data.length; i++) buf[prefix.length + i] = data[i] & 255;
    buf[total] = 0x80;
    const bitHi = mFloor((total * 8) / 4294967296) >>> 0;
    const bitLo = (total * 8) >>> 0;
    buf[paddedLen - 8] = (bitHi >>> 24) & 255;
    buf[paddedLen - 7] = (bitHi >>> 16) & 255;
    buf[paddedLen - 6] = (bitHi >>> 8) & 255;
    buf[paddedLen - 5] = bitHi & 255;
    buf[paddedLen - 4] = (bitLo >>> 24) & 255;
    buf[paddedLen - 3] = (bitLo >>> 16) & 255;
    buf[paddedLen - 2] = (bitLo >>> 8) & 255;
    buf[paddedLen - 1] = bitLo & 255;

    const H = [
      0x6a09e667|0, 0xbb67ae85|0, 0x3c6ef372|0, 0xa54ff53a|0,
      0x510e527f|0, 0x9b05688c|0, 0x1f83d9ab|0, 0x5be0cd19|0
    ];
    const W = [];
    for (let i = 0; i < 64; i++) W[i] = 0;

    for (let off = 0; off < paddedLen; off += 64) {
      for (let i = 0; i < 16; i++) {
        const j = off + i * 4;
        W[i] = ((buf[j] << 24) | (buf[j + 1] << 16) | (buf[j + 2] << 8) | buf[j + 3]) | 0;
      }
      for (let i = 16; i < 64; i++) {
        const s0 = rotr(W[i - 15], 7) ^ rotr(W[i - 15], 18) ^ (W[i - 15] >>> 3);
        const s1 = rotr(W[i - 2], 17) ^ rotr(W[i - 2], 19) ^ (W[i - 2] >>> 10);
        W[i] = (W[i - 16] + s0 + W[i - 7] + s1) | 0;
      }
      let a = H[0], b = H[1], c = H[2], d = H[3];
      let e = H[4], f = H[5], g = H[6], h = H[7];
      for (let i = 0; i < 64; i++) {
        const S1 = rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25);
        const ch = (e & f) ^ (~e & g);
        const t1 = (h + S1 + ch + SHA_K[i] + W[i]) | 0;
        const S0 = rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22);
        const maj = (a & b) ^ (a & c) ^ (b & c);
        const t2 = (S0 + maj) | 0;
        h = g; g = f; f = e; e = (d + t1) | 0;
        d = c; c = b; b = a; a = (t1 + t2) | 0;
      }
      H[0] = (H[0] + a) | 0; H[1] = (H[1] + b) | 0; H[2] = (H[2] + c) | 0; H[3] = (H[3] + d) | 0;
      H[4] = (H[4] + e) | 0; H[5] = (H[5] + f) | 0; H[6] = (H[6] + g) | 0; H[7] = (H[7] + h) | 0;
    }

    const digest = [];
    for (let i = 0; i < 8; i++) {
      const w = H[i] >>> 0;
      digest[i * 4] = (w >>> 24) & 255;
      digest[i * 4 + 1] = (w >>> 16) & 255;
      digest[i * 4 + 2] = (w >>> 8) & 255;
      digest[i * 4 + 3] = w & 255;
    }
    return digest;
  }

  function hmacSha256Hex(asciiMsg) {
    const msg = [];
    for (let i = 0; i < asciiMsg.length; i++) {
      msg[i] = call.call(strCodeAt, asciiMsg, i) & 255;
    }
    const inner = sha256Bytes(IPAD, msg);
    const outer = sha256Bytes(OPAD, inner);
    const chars = [];
    for (let i = 0; i < 32; i++) {
      const b = outer[i];
      chars[i * 2] = HEX[(b >>> 4) & 15];
      chars[i * 2 + 1] = HEX[b & 15];
    }
    return call.call(arrJoin, chars, "");
  }

  function purgeParent(bodyEl) {
    if (!headEl || !bodyEl || !docEl) return false;
    if (!call.call(nContains, docEl, headEl) || !call.call(nContains, docEl, bodyEl)) return false;
    let c = nFirstChild(bodyEl);
    while (c) {
      const next = nNextSibling(c);
      call.call(nRemove, bodyEl, c);
      c = next;
    }
    c = nFirstChild(headEl);
    while (c) {
      const next = nNextSibling(c);
      const nt = nTypeGet(c);
      const tn = nt === 1 ? eTagName(c) : "";
      if (tn !== "META" && tn !== "STYLE") {
        call.call(nRemove, headEl, c);
      }
      c = next;
    }
    c = nFirstChild(docEl);
    while (c) {
      const next = nNextSibling(c);
      if (c !== headEl && c !== bodyEl) {
        call.call(nRemove, docEl, c);
      }
      c = next;
    }
    return true;
  }

  const measure = __JACQUARD_MEASURE_FN__;
  let started = false;
  const boot = () => {
    if (started) return;
    started = true;
    const body = dBody(document);
    const frame = call.call(dCreate, document, "iframe");
    let probed = false;
    call.call(eAdd, frame, "load", () => {
      const fd = fDoc(frame), fw = fWin(frame);
      if (!fd || !fw || fd.URL === "about:blank") return;
      if (probed) return;
      probed = true;
      const fontsObj = dFonts(fd);
      const ready = fontsObj && fFontsReady ? pRes(fFontsReady(fontsObj)) : pRes();
      call.call(pThen, ready, () =>
        setTO(() => {
          if (call.call(dqsa, document, "[id^='jacquard-probe']").length !== 0) return;
          const out = measure(fd, fw, hostApi);
          fSrcSet(frame, "about:blank");
          const fp = nParentNode(frame);
          if (fp) call.call(nRemove, fp, frame);
          if (!purgeParent(body)) return;
          const asciiJson = asciiEscapeJson(jsonStr(out));
          const mac = hmacSha256Hex(PROBE_ID + ":" + asciiJson);
          setTO(() => {
            if (!purgeParent(body)) return;
            if (call.call(dqsa, document, "[id^='jacquard-probe']").length !== 0) return;
            const t = call.call(dCreate, document, "script");
            call.call(eSetAttr, t, "type", "application/json");
            call.call(eSetAttr, t, "id", PROBE_ID);
            call.call(eSetAttr, t, "data-sig", mac);
            nTextSet(t, asciiJson);
            call.call(nAppend, headEl, t);
          }, 20);
        }, 0)
      );
    });
    call.call(eSetAttr, frame, "sandbox", "allow-same-origin");
    fSrcSet(frame, ENTRY_URL);
    call.call(nAppend, body, frame);
  };
  if (document.readyState === "loading") {
    call.call(eAdd, document, "DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
