// Jacquard page renderer. Reads one JSON job on stdin, prints one JSON line
// per render. Measures and screenshots; never judges.
import { readFileSync } from "node:fs";

const job = JSON.parse(readFileSync(0, "utf8"));
const { chromium } = await import(job.playwright);
const probe = readFileSync(job.probe, "utf8");
const targetOrigin = new URL(job.url).origin;
const browser = await chromium.launch({
  executablePath: job.chrome,
  args: [
    "--use-angle=swiftshader",
    "--enable-unsafe-swiftshader",
    "--ignore-gpu-blocklist",
    "--enable-webgl",
    "--host-resolver-rules=MAP * ~NOTFOUND , EXCLUDE 127.0.0.1 , EXCLUDE localhost",
    "--proxy-server=direct://",
    "--disable-background-networking",
    "--disable-extensions",
  ],
});
try {
  for (const [width, height] of job.viewports) {
    const context = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 1, serviceWorkers: "block" });
    await context.route("**/*", (route) => {
      try {
        const reqUrl = new URL(route.request().url());
        if (reqUrl.origin === targetOrigin && (reqUrl.protocol === "http:" || reqUrl.protocol === "https:")) {
          return route.continue();
        }
      } catch {}
      return route.abort("blockedbyclient");
    });
    const page = await context.newPage();
    const out = { url: job.url, viewport: [width, height] };
    try {
      await page.goto(job.url, { waitUntil: "load", timeout: 60000 });
      await page.waitForFunction(() => {
        const s = document.querySelector("[data-scene-status]")?.textContent || "";
        return /webgl|webgpu|schematic/.test(s);
      }, null, { timeout: 60000 });
      await page.waitForTimeout(job.settle_ms);
      // Evaluated in a fresh CDP Isolated World: keeps product WASM/WebGL alive
      // while shielding probe.js from main-world prototype poisoning and DOM tampering.
      const cdp = await context.newCDPSession(page);
      const tree = await cdp.send("Page.getFrameTree");
      const frameId = tree.frameTree.frame.id;
      const { executionContextId } = await cdp.send("Page.createIsolatedWorld", {
        frameId,
        worldName: "jacquard-probe",
      });
      const evalRes = await cdp.send("Runtime.evaluate", {
        contextId: executionContextId,
        expression: `(${probe})(document, window, null)`,
        returnByValue: true,
      });
      if (evalRes.exceptionDetails) {
        const errText =
          evalRes.exceptionDetails.exception?.description ||
          evalRes.exceptionDetails.text ||
          "isolated probe evaluation failed";
        throw new Error(errText);
      }
      out.probe = evalRes.result.value;
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
