/**
 * R9: the particle engine and the three-beat timeline for AgentScene. Loaded only when the scene
 * comes near the viewport.
 *
 * Particles are sampled from the agent card's real call text (drawn off screen in the card's own
 * font), then follow two quadratic Bézier legs: card -> Runtime Core -> a point on the app's
 * screen. One canvas, a bounded pool in typed arrays (desktop 1,600, phones 450), no allocation per
 * frame, no blur or shadow; one GSAP tween per beat drives a single progress value the canvas
 * draws from. Frames are drawn only while a beat plays; the timeline pauses off screen.
 */
import { gsap } from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";

gsap.registerPlugin(ScrollTrigger);

export type SceneController = { replay: () => void; dispose: () => void };

type Pt = { x: number; y: number };

export function createScene(stage: HTMLElement, opts: { onDone: () => void }): SceneController {
  const q = <T extends Element = HTMLElement>(s: string) => stage.querySelector(s) as T;
  const qa = (s: string) => Array.from(stage.closest("figure")!.querySelectorAll<HTMLElement>(s));
  const canvas = q<HTMLCanvasElement>("[data-as-canvas]");
  const ctx = canvas.getContext("2d")!;
  const claude = q('[data-as-card="claude"]');
  const codex = q('[data-as-card="codex"]');
  const core = q("[data-as-core]");
  const glow = q("[data-as-glow]");
  const screen = q("[data-as-screen]");
  const flash = q("[data-as-flash]");
  const status = q("[data-as-status]");
  const uptime = q("[data-as-uptime]");
  const steps = qa("[data-as-step]");

  const small = window.matchMedia("(max-width: 767.98px)").matches;
  const N = small ? 450 : 1600;
  const sx = new Float32Array(N); // start (sampled from the text)
  const sy = new Float32Array(N);
  const cx = new Float32Array(N); // core pass point (jittered)
  const cy = new Float32Array(N);
  const tx = new Float32Array(N); // target on the screen, or the scatter end point
  const ty = new Float32Array(N);
  const k1x = new Float32Array(N); // leg controls
  const k1y = new Float32Array(N);
  const k2x = new Float32Array(N);
  const k2y = new Float32Array(N);
  const delay = new Float32Array(N);
  let mode: "travel" | "scatter" = "travel";
  let count = 0;
  const prog = { v: 0 };
  let w = 0;
  let h = 0;
  let dpr = 1;
  const color = getComputedStyle(stage).getPropertyValue("--band-accent").trim() || "#9daef7";

  function resize() {
    const r = stage.getBoundingClientRect();
    w = r.width;
    h = r.height;
    dpr = Math.min(window.devicePixelRatio || 1, 2);
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
  }

  const rel = (el: Element) => {
    const s = stage.getBoundingClientRect();
    const r = el.getBoundingClientRect();
    return { x: r.left - s.left, y: r.top - s.top, w: r.width, h: r.height };
  };

  /** sample points from the card's call text, drawn off screen in the card's own font */
  function sampleText(card: HTMLElement): Pt[] {
    const off = document.createElement("canvas");
    off.width = Math.ceil(w);
    off.height = Math.ceil(h);
    const o = off.getContext("2d", { willReadFrequently: true })!;
    o.fillStyle = "#fff";
    o.textBaseline = "top";
    for (const code of Array.from(card.querySelectorAll("code"))) {
      const b = rel(code);
      const cs = getComputedStyle(code);
      o.font = `${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
      o.fillText(code.textContent ?? "", b.x, b.y + 1);
    }
    const data = o.getImageData(0, 0, off.width, off.height).data;
    const pts: Pt[] = [];
    for (let y = 0; y < off.height; y += 1)
      for (let x = 0; x < off.width; x += 1) if (data[(y * off.width + x) * 4 + 3] > 120) pts.push({ x, y });
    return pts;
  }

  function prepare(card: HTMLElement, m: "travel" | "scatter") {
    mode = m;
    const pts = sampleText(card);
    if (pts.length === 0) {
      count = 0;
      return;
    }
    count = Math.min(N, pts.length);
    const c = rel(core);
    const sc = rel(screen);
    const ccx = c.x + c.w / 2;
    const ccy = c.y + c.h / 2;
    for (let i = 0; i < count; i++) {
      const p = pts[Math.floor(Math.random() * pts.length)];
      sx[i] = p.x;
      sy[i] = p.y;
      delay[i] = Math.random() * 0.35 + (p.x / w) * 0.1;
      if (m === "travel") {
        cx[i] = ccx + (Math.random() - 0.5) * c.w * 0.5;
        cy[i] = ccy + (Math.random() - 0.5) * c.h * 0.5;
        tx[i] = sc.x + 6 + Math.random() * (sc.w - 12);
        ty[i] = sc.y + 6 + Math.random() * (sc.h - 12);
        k1x[i] = (sx[i] + cx[i]) / 2;
        k1y[i] = Math.min(sy[i], cy[i]) - 30 - Math.random() * 60;
        k2x[i] = (cx[i] + tx[i]) / 2;
        k2y[i] = Math.max(cy[i], ty[i]) + 20 + Math.random() * 50;
      } else {
        const a = Math.random() * Math.PI * 2;
        const d = 20 + Math.random() * 70;
        tx[i] = sx[i] + Math.cos(a) * d - 25;
        ty[i] = sy[i] + Math.sin(a) * d + 30;
      }
    }
  }

  const ease = (t: number) => (t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2);
  const qb = (a: number, b: number, c: number, t: number) => (1 - t) * (1 - t) * a + 2 * (1 - t) * t * b + t * t * c;

  function draw() {
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = color;
    const span = 0.55;
    for (let i = 0; i < count; i++) {
      let t = (prog.v - delay[i]) / span;
      if (t <= 0 || t >= 1) continue;
      t = ease(t);
      let x: number;
      let y: number;
      let a: number;
      if (mode === "travel") {
        if (t < 0.5) {
          const u = t * 2;
          x = qb(sx[i], k1x[i], cx[i], u);
          y = qb(sy[i], k1y[i], cy[i], u);
        } else {
          const u = (t - 0.5) * 2;
          x = qb(cx[i], k2x[i], tx[i], u);
          y = qb(cy[i], k2y[i], ty[i], u);
        }
        a = t < 0.1 ? t * 10 : t > 0.88 ? (1 - t) / 0.12 : 1;
      } else {
        x = sx[i] + (tx[i] - sx[i]) * t;
        y = sy[i] + (ty[i] - sy[i]) * t;
        a = 1 - t;
      }
      ctx.globalAlpha = a;
      ctx.fillRect(x, y, 1.6, 1.6);
    }
    ctx.globalAlpha = 1;
  }

  /** one beat of particles: card text dissolves while they fly (or scatter) */
  function particles(tl: gsap.core.Timeline, card: HTMLElement, m: "travel" | "scatter", at: number, dur: number) {
    tl.call(() => prepare(card, m), [], at)
      .fromTo(prog, { v: 0 }, { v: 1, duration: dur, ease: "none", onUpdate: draw, immediateRender: false }, at)
      .to(card.querySelectorAll("li"), { opacity: m === "travel" ? 0.12 : 0, duration: 0.35, stagger: 0.06, ease: "power1.in" }, at)
      .call(() => ctx.clearRect(0, 0, canvas.width, canvas.height), [], at + dur);
  }

  // uptime keeps counting while the scene is on screen (illustrative readout)
  let secs = 6 * 3600 + 21 * 60;
  let onScreen = false;
  const tick = window.setInterval(() => {
    if (!onScreen || document.hidden) return;
    secs++;
    const hh = Math.floor(secs / 3600);
    const mm = String(Math.floor((secs % 3600) / 60)).padStart(2, "0");
    const ss = String(secs % 60).padStart(2, "0");
    uptime.textContent = `up ${hh}:${mm}:${ss}`;
  }, 1000);

  const activeStep = (n: number) => () => steps.forEach((s) => s.classList.toggle("as-step-on", s.dataset.asStep === String(n)));
  const label = (card: HTMLElement, text: string) => () => {
    card.querySelector("[data-as-label]")!.textContent = text;
  };

  function build() {
    const tl = gsap.timeline({ paused: true, onComplete: () => (steps.forEach((s) => s.classList.remove("as-step-on")), opts.onDone()) });
    // start state: Claude Code connected, Codex not there yet
    tl.set(codex, { autoAlpha: 0 }, 0)
      .set(claude, { autoAlpha: 1, filter: "none" }, 0)
      .set(claude.querySelectorAll("li"), { opacity: 1 }, 0)
      .call(label(claude, "Claude Code, connected"), [], 0)
      .call(label(codex, "Codex, connected"), [], 0)
      .set(glow, { opacity: 0 }, 0)
      .call(() => (status.textContent = "Running"), [], 0)
      .call(activeStep(1), [], 0);

    // beat 1: the agent acts
    particles(tl, claude, "travel", 0.4, 2.0);
    tl.to(glow, { opacity: 1, duration: 0.4, ease: "power2.out" }, 1.0)
      .fromTo(flash, { opacity: 0 }, { opacity: 0.55, duration: 0.25, yoyo: true, repeat: 1 }, 2.1)
      .call(() => (status.textContent = "Running"), [], 2.3)
      .to(claude.querySelectorAll("li"), { opacity: 1, duration: 0.4 }, 2.5);

    // beat 2: the agent leaves; the Core and the app stay
    tl.call(activeStep(2), [], 3.2);
    particles(tl, claude, "scatter", 3.3, 1.2);
    tl.call(label(claude, "Claude Code, disconnected"), [], 3.4)
      .to(claude, { autoAlpha: 0.45, filter: "grayscale(1)", duration: 0.5 }, 3.4)
      .to(glow, { opacity: 0.75, duration: 0.4 }, 3.4);

    // beat 3: another agent, the same session
    tl.call(activeStep(3), [], 4.9)
      .to(claude, { autoAlpha: 0, duration: 0.35 }, 4.9)
      .fromTo(codex, { autoAlpha: 0, y: 8 }, { autoAlpha: 1, y: 0, duration: 0.45, ease: "power3.out" }, 5.1)
      .set(codex.querySelectorAll("li"), { opacity: 1 }, 5.1);
    particles(tl, codex, "travel", 5.8, 1.9);
    tl.to(glow, { opacity: 1, duration: 0.3 }, 6.4)
      .fromTo(flash, { opacity: 0 }, { opacity: 0.55, duration: 0.25, yoyo: true, repeat: 1 }, 7.4)
      .call(() => (status.textContent = "Running, reloaded"), [], 7.6)
      .to(codex.querySelectorAll("li"), { opacity: 1, duration: 0.4 }, 7.8)
      .to(glow, { opacity: 0.6, duration: 0.6 }, 8.2);
    return tl;
  }

  resize();
  let tl = build();
  const ro = new ResizeObserver(() => {
    resize();
    if (!tl.isActive()) ctx.clearRect(0, 0, canvas.width, canvas.height);
  });
  ro.observe(stage);

  const st = ScrollTrigger.create({
    trigger: stage,
    start: "top 70%",
    end: "bottom top",
    onEnter: () => {
      onScreen = true;
      if (tl.progress() === 0) tl.play();
      else if (tl.progress() < 1) tl.resume();
    },
    onEnterBack: () => {
      onScreen = true;
      if (tl.progress() > 0 && tl.progress() < 1) tl.resume();
    },
    onLeave: () => {
      onScreen = false;
      tl.pause();
    },
    onLeaveBack: () => {
      onScreen = false;
      tl.pause();
    },
  });

  return {
    replay() {
      tl.kill();
      tl = build();
      tl.play();
    },
    dispose() {
      st.kill();
      tl.kill();
      ro.disconnect();
      clearInterval(tick);
    },
  };
}
