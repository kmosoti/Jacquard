// Jacquard page renderer. Reads one JSON job on stdin, prints one JSON line
// per render. Measures and screenshots; never judges.
import { readFileSync } from "node:fs";

const job = JSON.parse(readFileSync(0, "utf8"));
const { chromium } = await import(job.playwright);
const probe = readFileSync(job.probe, "utf8");
const browser = await chromium.launch({
  executablePath: job.chrome,
  args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist", "--enable-webgl"],
});
try {
  for (const [width, height] of job.viewports) {
    const context = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 1, serviceWorkers: "block" });
    const page = await context.newPage();
    const out = { url: job.url, viewport: [width, height] };
    try {
      await page.goto(job.url, { waitUntil: "load", timeout: 60000 });
      await page.waitForFunction(() => {
        const s = document.querySelector("[data-scene-status]")?.textContent || "";
        return /webgl|webgpu|schematic/.test(s);
      }, null, { timeout: 60000 });
      await page.waitForTimeout(job.settle_ms);
      // Evaluated over the DevTools protocol: a strict page CSP blocks an
      // inline script tag, and the probe only reads the page.
      await page.evaluate(probe);
      await page.waitForSelector("#jacquard-probe", { state: "attached", timeout: 30000 });
      out.probe = JSON.parse(await page.$eval("#jacquard-probe", (e) => e.textContent));
      out.png = `${job.out_prefix}-${width}x${height}.png`;
      await page.screenshot({ path: out.png });
    } catch (e) {
      out.error = String(e && e.message ? e.message : e).split("\n")[0];
    }
    console.log(JSON.stringify(out));
    await context.close();
  }
} finally {
  await browser.close();
}
