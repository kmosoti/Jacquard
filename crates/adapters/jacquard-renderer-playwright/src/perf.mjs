// Jacquard performance driver (milestone 05 protocol, P1 and P2). Reads one
// JSON job on stdin; prints one JSON line per run. A cold context per run,
// service workers allowed (their precache is a real first-visit cost).
import { readFileSync } from "node:fs";

process.env.PW_EXPERIMENTAL_SERVICE_WORKER_NETWORK_EVENTS = "1";
const job = JSON.parse(readFileSync(0, "utf8"));
const { chromium } = await import(job.playwright);
const observe = () => {
  window.__perf = { lcp: 0, cls: 0, longtasks: [], fcp: 0 };
  new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__perf.lcp = e.renderTime || e.loadTime || e.startTime; }).observe({ type: "largest-contentful-paint", buffered: true });
  new PerformanceObserver((l) => { for (const e of l.getEntries()) if (!e.hadRecentInput) window.__perf.cls += e.value; }).observe({ type: "layout-shift", buffered: true });
  new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__perf.longtasks.push([e.startTime, e.duration]); }).observe({ type: "longtask", buffered: true });
  new PerformanceObserver((l) => { for (const e of l.getEntries()) if (e.name === "first-contentful-paint") window.__perf.fcp = e.startTime; }).observe({ type: "paint", buffered: true });
};
for (let run = 0; run < job.runs; run++) {
  const browser = await chromium.launch({
    executablePath: job.chrome,
    args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist", "--enable-webgl"],
  });
  const out = { url: job.url, profile: job.profile, run };
  try {
    const context = await browser.newContext({ viewport: { width: job.viewport[0], height: job.viewport[1] }, deviceScaleFactor: 1 });
    let bytes = 0, requests = 0, inflight = 0, last = Date.now();
    context.on("request", () => { inflight++; requests++; last = Date.now(); });
    const done = async (r) => {
      inflight = Math.max(0, inflight - 1); last = Date.now();
      try { const s = await r.sizes(); bytes += s.responseBodySize + s.responseHeadersSize; } catch {}
    };
    context.on("requestfinished", done);
    context.on("requestfailed", () => { inflight = Math.max(0, inflight - 1); last = Date.now(); });
    const page = await context.newPage();
    await page.addInitScript(observe);
    if (job.throttle) {
      const cdp = await context.newCDPSession(page);
      await cdp.send("Network.enable");
      await cdp.send("Network.emulateNetworkConditions", { offline: false, latency: 150, downloadThroughput: (1.6e6 / 8), uploadThroughput: (750e3 / 8) });
      await cdp.send("Emulation.setCPUThrottlingRate", { rate: 4 });
    }
    const t0 = Date.now();
    await page.goto(job.url, { waitUntil: "load", timeout: 180000 });
    out.load_ms = Date.now() - t0;
    // Quiet: no request in flight and none started for 3 s, capped at 90 s.
    const cap = Date.now() + 90000;
    while (Date.now() < cap && (inflight > 0 || Date.now() - last < 3000)) await page.waitForTimeout(250);
    const p = await page.evaluate(() => window.__perf);
    out.lcp_ms = Math.round(p.lcp);
    out.cls = Math.round(p.cls * 1000) / 1000;
    out.tbt_ms = Math.round(p.longtasks.filter(([s]) => s >= p.fcp).reduce((t, [, d]) => t + Math.max(0, d - 50), 0));
    out.bytes = bytes;
    out.requests = requests;
    await context.close();
  } catch (e) {
    out.error = String(e && e.message ? e.message : e).split("\n")[0];
  }
  console.log(JSON.stringify(out));
  await browser.close();
}
