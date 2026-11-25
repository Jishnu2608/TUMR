/**
 * WEB-017: the device lab's worker. It owns the OffscreenCanvas the page transferred, loads
 * three.js and the screens, builds the devices, runs the demo and draws every frame, so none of
 * that runs on the page's main thread (scrolling, taps and Lighthouse's blocking time stay clear).
 * The page sends sizes, visibility and the visitor's commands; the worker sends back the lab's
 * state, the first frame, the switch fade and a lost GPU context.
 */
import { createLab, type DeviceId, type Lab } from "./scene";

export type ToWorker =
  | { type: "init"; canvas: OffscreenCanvas; w: number; h: number; dpr: number; reduced: boolean; textureBase: string }
  | { type: "resize"; w: number; h: number }
  | { type: "visible"; v: boolean }
  | { type: "show"; id: DeviceId }
  | { type: "hinge"; deg: number }
  | { type: "turn"; dYaw: number; dPitch: number }
  | { type: "flip" }
  | { type: "reset" }
  | { type: "demo" }
  | { type: "stopDemo" };

export type FromWorker =
  | { type: "state"; device: DeviceId; hinge: number; back: boolean; demo: boolean }
  | { type: "first" }
  | { type: "fade"; o: number | null; ms?: number; ease?: string }
  | { type: "lost" };

const post = (m: FromWorker) => (self as unknown as Worker).postMessage(m);
let lab: Lab | null = null;
// state goes to the page only when something it shows changes (not on every animation frame)
let last = "";
const postState = (s: { device: DeviceId; hinge: number; back: boolean; demo: boolean }) => {
  const key = `${s.device}|${Math.round(s.hinge)}|${s.back}|${s.demo}`;
  if (key === last) return;
  last = key;
  post({ type: "state", ...s, hinge: Math.round(s.hinge) });
};

self.onmessage = (e: MessageEvent<ToWorker>) => {
  const m = e.data;
  if (m.type === "init") {
    // no WebGL in the worker (the page no longer probes first): report it, the picture stays
    try {
      lab = createLab(m.canvas, {
        reduced: m.reduced,
        textureBase: m.textureBase,
        dpr: m.dpr,
        onState: postState,
        onFirstFrame: () => post({ type: "first" }),
        onFade: (o, ms, ease) => post({ type: "fade", o, ms, ease }),
        onLost: () => post({ type: "lost" }),
      });
    } catch {
      post({ type: "lost" });
      return;
    }
    lab.start(m.w, m.h).catch(() => post({ type: "lost" }));
    return;
  }
  if (!lab) return;
  switch (m.type) {
    case "resize":
      lab.resize(m.w, m.h);
      break;
    case "visible":
      lab.setVisible(m.v);
      break;
    case "show":
      lab.stopDemo();
      void lab.show(m.id);
      break;
    case "hinge":
      lab.stopDemo();
      void lab.setHinge(m.deg);
      break;
    case "turn":
      lab.stopDemo();
      lab.turn(m.dYaw, m.dPitch);
      break;
    case "flip":
      lab.flipSide();
      break;
    case "reset":
      lab.reset();
      break;
    case "demo":
      void lab.demo();
      break;
    case "stopDemo":
      lab.stopDemo();
      break;
  }
};
