// R8 (hero device lab): renders the sample travel app's screens to WebP textures for the 3D devices.
// The screens are the same HTML/CSS as the hero showcase (components/home/SampleTravelApp.tsx), laid
// out at each device's own screen size, so every device shows a layout made for it (nothing is
// stretched). Re-run after changing the sample app or the device screen sizes.
//
// usage: start the dev server (npm run dev, port 27310) and a Chrome with
//        --remote-debugging-port=9333, then: node scripts/capture-device-screens.mjs
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const OUT = join(import.meta.dirname, "..", "public", "images", "device-lab");
const URL = process.env.SITE ?? "http://localhost:27310/";
mkdirSync(OUT, { recursive: true });

// width x height in the showcase's screen units; px = pixel width of the texture
const shots = [
  { name: "standard", from: ".dv-phone .ta", u: [220, 470], px: 880 },
  { name: "compact", from: ".dv-compact .ta", u: [190, 388], px: 800 },
  { name: "tablet", from: ".dv-tablet .ta", u: [530, 364], px: 1600 },
  { name: "fold-left", from: ".dv-panel-left .ta", u: [193, 422], px: 780 },
  { name: "fold-right", from: ".dv-face-front .ta", u: [193, 422], px: 780 },
  { name: "fold-cover", from: ".dv-face-back .ta", u: [186, 422], px: 760 },
  { name: "flip-inner", from: ".dv-phone .ta", u: [214, 520], px: 860 },
  { name: "flip-cover", html: "flipCover", u: [150, 128], px: 600 },
];

const flipCover = `
<div class="ta" style="padding:calc(10*var(--u)) calc(12*var(--u));gap:calc(6*var(--u));justify-content:space-between">
  <div class="ta-status" style="padding:0"><span>9:41</span><span class="ta-status-icons"><i></i><i class="ta-batt"></i></span></div>
  <div style="display:grid;grid-template-columns:calc(46*var(--u)) 1fr;gap:calc(10*var(--u));align-items:center">
    <img src="/images/sample-app/lisbon.webp" alt="" style="width:100%;height:calc(46*var(--u));object-fit:cover;border-radius:calc(10*var(--u))">
    <div style="display:grid;gap:calc(2*var(--u))">
      <span class="ta-hello">Next trip</span>
      <span class="ta-title-s ta-title">Lisbon</span>
      <span class="ta-small-meta">In 3 days</span>
    </div>
  </div>
  <span class="ta-cta" style="margin-top:0;height:calc(24*var(--u))">Open itinerary</span>
</div>`;

const list = await (await fetch("http://127.0.0.1:9333/json/list")).json();
const ws = new WebSocket(list.find((t) => t.type === "page").webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener("open", r, { once: true }));
let id = 0;
const pending = new Map();
ws.addEventListener("message", (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pending.has(m.id)) {
    pending.get(m.id)(m);
    pending.delete(m.id);
  }
});
const send = (method, params = {}) =>
  new Promise((res) => {
    const n = ++id;
    pending.set(n, res);
    ws.send(JSON.stringify({ id: n, method, params }));
  });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const ev = async (x) => (await send("Runtime.evaluate", { expression: x, returnByValue: true, awaitPromise: true })).result.result?.value;

await send("Emulation.setDeviceMetricsOverride", { width: 2200, height: 2800, deviceScaleFactor: 1, mobile: false });
await send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-motion", value: "reduce" }, { name: "prefers-color-scheme", value: "light" }] });
await send("Page.navigate", { url: URL });
await sleep(3500);

for (const s of shots) {
  const unit = s.px / s.u[0];
  const h = Math.round(s.u[1] * unit);
  const ok = await ev(`(async () => {
    document.getElementById("__cap")?.remove();
    if (!document.getElementById("__capstyle")) { const st = document.createElement("style"); st.id = "__capstyle"; st.textContent = "nextjs-portal{display:none!important}"; document.head.appendChild(st); }
    const box = document.createElement("div");
    box.id = "__cap";
    box.style.cssText = "position:fixed;left:0;top:0;z-index:99999;overflow:hidden;width:${s.px}px;height:${h}px;--u:${unit}px;background:var(--app-bg)";
    ${s.html ? `box.innerHTML = ${JSON.stringify(flipCover)};` : `const src = document.querySelector(${JSON.stringify(s.from)}); if (!src) return false; box.appendChild(src.cloneNode(true));`}
    document.body.appendChild(box);
    box.querySelectorAll("img").forEach((i) => { i.loading = "eager"; });
    await Promise.all([...box.querySelectorAll("img")].map((i) => i.complete ? 0 : new Promise((r) => { i.onload = i.onerror = r; })));
    await document.fonts.ready;
    return true;
  })()`);
  if (!ok) throw new Error(`no source for ${s.name}`);
  await sleep(300);
  const shot = await send("Page.captureScreenshot", { format: "webp", quality: 84, clip: { x: 0, y: 0, width: s.px, height: h, scale: 1 } });
  writeFileSync(join(OUT, `${s.name}.webp`), Buffer.from(shot.result.data, "base64"));
  console.log(`${s.name}.webp ${s.px}x${h}`);
}
await ev(`document.getElementById("__cap")?.remove()`);
ws.close();
