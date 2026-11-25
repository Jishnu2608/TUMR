"use client";

import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";
import { RotateCcw, Repeat2 } from "lucide-react";
import { prefersReducedMotion } from "@/design-system/motion";
import { cn } from "@/lib/cn";
import type { DeviceId, LabState } from "./lab/scene";
import type { FromWorker, ToWorker } from "./lab/worker";

/** What the page can ask of the lab, whether it runs in the worker or (fallback) on the page. */
type Control = {
  show: (id: DeviceId) => void;
  hinge: (deg: number) => void;
  turn: (dYaw: number, dPitch: number) => void;
  flip: () => void;
  reset: () => void;
  stopDemo: () => void;
};

const DEVICES: { id: DeviceId; label: string }[] = [
  { id: "standard", label: "Standard" },
  { id: "compact", label: "Compact" },
  { id: "tablet", label: "Tablet" },
  { id: "fold", label: "Fold" },
  { id: "flip", label: "Flip" },
];

/**
 * R8, in the hero: the device lab. The static picture (the R5 showcase's standard phone, from the
 * server) shows first; once the page has finished loading and the browser is idle, three.js and the
 * screen textures load and the 3D lab cross-fades in over the same area, on every screen size. It
 * then plays one short demo (each profile, both hinges) and hands control to the visitor: pick a
 * profile, drag (or arrow keys) to turn, turn it around, move the hinge of the fold and the flip.
 * Each control shows the real `tumr` command it corresponds to. Without WebGL, or if the GPU context
 * is lost, the static picture stays. React state changes only on discrete events (profile, side,
 * demo); the hinge readout is written to the DOM directly while it moves.
 *
 * WEB-017: the lab runs in a Web Worker on an OffscreenCanvas (lab/worker.ts), so three.js, the
 * device builds, the demo and every frame stay off the page's main thread. This component only
 * transfers the canvas, reports size and visibility, forwards the visitor's input, and applies the
 * state and the switch fade the worker sends back. Browsers without OffscreenCanvas WebGL run the
 * same scene on the page (the R8 behaviour).
 */
export function DeviceLab({ poster }: { poster: ReactNode }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const ctl = useRef<Control | null>(null);
  const sliderRef = useRef<HTMLInputElement>(null);
  const angleRef = useRef<HTMLElement>(null);
  const [ready, setReady] = useState(false);
  const [failed, setFailed] = useState(false);
  const [ui, setUi] = useState<Omit<LabState, "hinge">>({ device: "standard", back: false, demo: false });

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    let cancelled = false;
    let cleanup: (() => void) | null = null;

    const writeHinge = (deg: number) => {
      const v = String(Math.round(deg));
      if (sliderRef.current && sliderRef.current.value !== v) sliderRef.current.value = v;
      if (angleRef.current && angleRef.current.textContent !== v) angleRef.current.textContent = v;
    };
    // React renders only when what it shows changes (the hinge readout is written directly)
    let shown = "";
    const onState = (s: LabState) => {
      writeHinge(s.hinge);
      const key = `${s.device}|${s.back}|${s.demo}`;
      if (key === shown) return;
      shown = key;
      setUi({ device: s.device, back: s.back, demo: s.demo });
    };
    // one CSS transition per fade phase (the compositor runs it; no per-frame main-thread work)
    const onFade = (o: number | null, ms = 0, ease = "linear") => {
      canvas.style.transition = o === null ? "" : `opacity ${ms}ms ${ease}`;
      canvas.style.opacity = o === null ? "" : String(o);
    };
    const size = () => ({ w: Math.max(1, canvas.clientWidth), h: Math.max(1, canvas.clientHeight) });
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const reduced = prefersReducedMotion();

    /** wiring shared by both modes: size, visibility, the first frame, the demo */
    const host = (resize: (w: number, h: number) => void, visible: (v: boolean) => void, demo: () => void) => {
      let onScreen = true;
      const sync = () => visible(onScreen && !document.hidden);
      const io = new IntersectionObserver(([e]) => {
        onScreen = e.isIntersecting;
        sync();
      });
      io.observe(canvas);
      document.addEventListener("visibilitychange", sync);
      const ro = new ResizeObserver(() => {
        const { w, h } = size();
        resize(w, h);
      });
      ro.observe(canvas);
      const first = () => {
        if (cancelled) return;
        setReady(true);
        if (!reduced) window.setTimeout(() => !cancelled && demo(), 900);
      };
      return {
        first,
        stop: () => {
          io.disconnect();
          ro.disconnect();
          document.removeEventListener("visibilitychange", sync);
        },
      };
    };

    /** the worker path: everything heavy off the main thread */
    const bootWorker = () => {
      const worker = new Worker(new URL("./lab/worker.ts", import.meta.url));
      const send = (m: ToWorker, transfer: Transferable[] = []) => worker.postMessage(m, transfer);
      const h = host(
        (w, hh) => send({ type: "resize", w, h: hh }),
        (v) => send({ type: "visible", v }),
        () => send({ type: "demo" }),
      );
      worker.onmessage = (e: MessageEvent<FromWorker>) => {
        const m = e.data;
        if (m.type === "state") onState(m);
        else if (m.type === "first") h.first();
        else if (m.type === "fade") onFade(m.o, m.ms, m.ease);
        else if (m.type === "lost") setFailed(true);
      };
      worker.onerror = () => setFailed(true);
      const offscreen = canvas.transferControlToOffscreen();
      canvas.dataset.offscreen = "1";
      const { w, h: hh } = size();
      send({ type: "init", canvas: offscreen, w, h: hh, dpr, reduced, textureBase: new URL("/images/device-lab/", location.href).href }, [offscreen]);
      ctl.current = {
        show: (id) => send({ type: "show", id }),
        hinge: (deg) => send({ type: "hinge", deg }),
        turn: (dYaw, dPitch) => send({ type: "turn", dYaw, dPitch }),
        flip: () => send({ type: "flip" }),
        reset: () => send({ type: "reset" }),
        stopDemo: () => send({ type: "stopDemo" }),
      };
      cleanup = () => {
        h.stop();
        worker.terminate();
      };
    };

    /** the fallback: the same scene on the page */
    const bootPage = async () => {
      const { createLab } = await import("./lab/scene");
      if (cancelled) return;
      let first = () => {};
      const lab = createLab(canvas, {
        reduced,
        textureBase: "/images/device-lab/",
        dpr,
        onState,
        onFirstFrame: () => first(),
        onFade,
        onLost: () => setFailed(true),
      });
      const h = host(lab.resize, lab.setVisible, () => void lab.demo());
      first = h.first;
      ctl.current = {
        show: (id) => {
          lab.stopDemo();
          void lab.show(id);
        },
        hinge: (deg) => {
          lab.stopDemo();
          void lab.setHinge(deg);
        },
        turn: (dYaw, dPitch) => {
          lab.stopDemo();
          lab.turn(dYaw, dPitch);
        },
        flip: lab.flipSide,
        reset: lab.reset,
        stopDemo: lab.stopDemo,
      };
      cleanup = () => {
        h.stop();
        lab.dispose();
      };
      const { w, h: hh } = size();
      await lab.start(w, hh);
    };

    const boot = () => {
      if (cancelled || canvas.dataset.offscreen) return;
      // No WebGL probe on the page: creating a throwaway context costs the main thread hundreds of
      // milliseconds on a slow phone. The worker makes the only context; if it cannot, it reports
      // "lost" and the static picture stays.
      if (typeof OffscreenCanvas !== "undefined" && "transferControlToOffscreen" in canvas) {
        try {
          bootWorker();
          return;
        } catch {
          /* fall through to the page */
        }
      }
      // older browsers: WebGL available? if not, the static picture stays
      const probe = document.createElement("canvas");
      if (!(probe.getContext("webgl2") || probe.getContext("webgl"))) return;
      void bootPage();
    };

    // wait for the page to finish loading, then for an idle moment
    let idleId = 0;
    const whenIdle = () => {
      const ric = (window as Window & { requestIdleCallback?: (cb: () => void, o?: { timeout: number }) => number }).requestIdleCallback;
      if (ric) idleId = ric(boot, { timeout: 1500 });
      else idleId = window.setTimeout(boot, 300);
    };
    // after the load event, a short pause (the headline and buttons settle first), then idle
    let delay = 0;
    const afterLoad = () => {
      delay = window.setTimeout(whenIdle, 1200);
    };
    if (document.readyState === "complete") afterLoad();
    else window.addEventListener("load", afterLoad, { once: true });

    return () => {
      cancelled = true;
      window.removeEventListener("load", afterLoad);
      clearTimeout(delay);
      const cic = (window as Window & { cancelIdleCallback?: (id: number) => void }).cancelIdleCallback;
      if (cic) cic(idleId);
      clearTimeout(idleId);
      cleanup?.();
      ctl.current = null;
    };
  }, []);

  // ---- input
  const drag = useRef<{ x: number; y: number; id: number } | null>(null);
  const onDown = (e: PointerEvent<HTMLCanvasElement>) => {
    if (!ctl.current) return;
    ctl.current.stopDemo();
    drag.current = { x: e.clientX, y: e.clientY, id: e.pointerId };
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const onMove = (e: PointerEvent<HTMLCanvasElement>) => {
    const d = drag.current;
    if (!d || d.id !== e.pointerId || !ctl.current) return;
    ctl.current.turn((e.clientX - d.x) * 0.009, (e.clientY - d.y) * 0.006);
    d.x = e.clientX;
    d.y = e.clientY;
  };
  const onUp = () => {
    drag.current = null;
  };
  const onKey = (e: KeyboardEvent<HTMLCanvasElement>) => {
    if (!ctl.current) return;
    const step = 0.26;
    const map: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step / 2], ArrowDown: [0, step / 2] };
    const m = map[e.key];
    if (!m) return;
    e.preventDefault();
    ctl.current.turn(m[0], m[1]);
  };

  const pick = (id: DeviceId) => ctl.current?.show(id);
  const foldable = ui.device === "fold" || ui.device === "flip";
  const live = ready && !failed;

  return (
    <figure className="lab" aria-label="Device lab: a sample travel app on the runtime's device profiles, in 3D">
      <div className="lab-stage" data-hero-visual>
        <div className={cn("lab-poster", live && "lab-poster-off")}>{poster}</div>
        <canvas
          ref={canvasRef}
          className={cn("lab-canvas", live && "lab-canvas-on")}
          tabIndex={live ? 0 : -1}
          role="img"
          aria-label={`${DEVICES.find((d) => d.id === ui.device)?.label} device profile, ${ui.back ? "back" : "front"}. Drag or use the arrow keys to turn it.`}
          onPointerDown={onDown}
          onPointerMove={onMove}
          onPointerUp={onUp}
          onPointerCancel={onUp}
          onKeyDown={onKey}
        />
      </div>

      <figcaption className={cn("lab-controls", live && "lab-controls-on")} aria-hidden={!live}>
        <div className="lab-row">
          <div className="lab-chips" role="group" aria-label="Device profile">
            {DEVICES.map((d) => (
              <button
                key={d.id}
                type="button"
                className={cn("lab-chip", ui.device === d.id && "lab-chip-on")}
                aria-pressed={ui.device === d.id}
                tabIndex={live ? 0 : -1}
                onClick={() => pick(d.id)}
              >
                {d.label}
              </button>
            ))}
          </div>
          <div className="lab-actions">
            <button type="button" className="lab-icon" tabIndex={live ? 0 : -1} onClick={() => ctl.current?.flip()} aria-label={ui.back ? "Show the front" : "Show the back"} title={ui.back ? "Show the front" : "Show the back"}>
              <Repeat2 aria-hidden className="size-4" />
            </button>
            <button type="button" className="lab-icon" tabIndex={live ? 0 : -1} onClick={() => ctl.current?.reset()} aria-label="Reset the view" title="Reset the view">
              <RotateCcw aria-hidden className="size-4" />
            </button>
          </div>
        </div>
        <div className="lab-row lab-row-cmd">
          <code className="lab-cmd">tumr device use {ui.device}</code>
          <label className={cn("lab-hinge", !foldable && "lab-hinge-off")}>
            <span className="lab-hinge-label">Hinge</span>
            <input
              ref={sliderRef}
              type="range"
              min={0}
              max={180}
              step={1}
              defaultValue={180}
              tabIndex={live && foldable ? 0 : -1}
              aria-label="Hinge angle in degrees"
              onInput={(e) => ctl.current?.hinge(Number(e.currentTarget.value))}
            />
            <code className="lab-cmd">
              --angle <span ref={angleRef}>180</span>
            </code>
          </label>
        </div>
      </figcaption>
      <p className="lab-credit">
        Sample app for illustration. Photos:{" "}
        <a title="CC BY 4.0" href="https://commons.wikimedia.org/wiki/File:Tram_28_at_Largo_de_Santa_Luzia_in_Lisbon,_20250606_1958_0419.jpg" rel="noreferrer">
          Jakub Hałun
        </a>
        ,{" "}
        <a title="CC BY 3.0" href="https://commons.wikimedia.org/wiki/File:Herbert_Lake_beim_Icefields_Parkway.jpg" rel="noreferrer">
          Florian Fuchs
        </a>
        ,{" "}
        <a
          title="CC BY-SA 4.0"
          href="https://commons.wikimedia.org/wiki/File:Wooden_gate_with_open_double_door_leading_to_the_garden_of_Isshinin_Buddhist_temple_in_the_compounds_of_Chion-in_Kyoto_Japan.jpg"
          rel="noreferrer"
        >
          Basile Morin
        </a>
        ,{" "}
        <a title="Public domain" href="https://commons.wikimedia.org/wiki/File:Utah_Dunes_Landscape_-_West_Desert_District.jpg" rel="noreferrer">
          BLM Utah
        </a>{" "}
        (Wikimedia Commons)
      </p>
    </figure>
  );
}
