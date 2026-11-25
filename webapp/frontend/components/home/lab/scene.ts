/**
 * R8 in the hero: the device laboratory scene (three.js, loaded only after the page has loaded).
 *
 * Five device profiles, modelled from simple shapes: standard, compact, tablet, fold (book fold,
 * vertical hinge) and flip (clamshell, horizontal hinge). Bodies are extruded rounded rectangles
 * with a bevelled metal edge; screens are separate rounded planes carrying textures rendered from
 * the sample app at each device's own screen size (scripts/capture-device-screens.mjs), so no
 * layout is stretched. Hinges turn on the front surface line, with a cylindrical spine that turns
 * at half the fold angle, so the device stays closed-form (no gaps) at every angle.
 *
 * Rendering is on demand: a frame is drawn only while something moves (a tween, a drag settling)
 * and never while the canvas is off screen or the tab is hidden.
 *
 * WEB-017: the scene is DOM-free, so it runs in a Web Worker on an OffscreenCanvas (lab/worker.ts)
 * and the page's main thread stays free; DeviceLab falls back to running it on the page where
 * OffscreenCanvas is missing. The host reports size, pixel ratio and visibility, and applies the
 * device-switch fade (onFade) to the canvas element; textures decode as ImageBitmaps.
 */
import * as THREE from "three";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";
import { gsap } from "gsap";

export type DeviceId = "standard" | "compact" | "tablet" | "fold" | "flip";
export const DEVICE_IDS: DeviceId[] = ["standard", "compact", "tablet", "fold", "flip"];
export const FOLDABLE = new Set<DeviceId>(["fold", "flip"]);

export type LabState = { device: DeviceId; hinge: number; back: boolean; demo: boolean };
export type LabOpts = {
  reduced: boolean;
  textureBase: string;
  dpr: number;
  onState: (s: LabState) => void;
  onFirstFrame: () => void;
  onLost: () => void;
  /**
   * The canvas fade during a device switch, once per phase: the host sets a CSS opacity
   * transition to `opacity` over `ms` with the CSS `ease`, so the compositor runs it and the page
   * gets two messages per switch instead of one per frame. null = back to the stylesheet.
   */
  onFade: (opacity: number | null, ms?: number, ease?: string) => void;
};

const TEX: Record<string, string> = {
  standard: "standard.webp",
  compact: "compact.webp",
  tablet: "tablet.webp",
  foldLeft: "fold-left.webp",
  foldRight: "fold-right.webp",
  foldCover: "fold-cover.webp",
  flipInner: "flip-inner.webp",
  flipCover: "flip-cover.webp",
};
// texture pixel sizes (width, height) for the screen aspect ratios
const PX: Record<string, [number, number]> = {
  standard: [880, 1880],
  compact: [800, 1634],
  tablet: [1600, 1099],
  foldLeft: [780, 1705],
  foldRight: [780, 1705],
  foldCover: [760, 1724],
  flipInner: [860, 2090],
  flipCover: [600, 512],
};
/* ---------------------------------------------------------------- geometry helpers */

type Radii = [number, number, number, number]; // top-left, top-right, bottom-right, bottom-left

function roundedRect(w: number, h: number, r: Radii) {
  const [tl, tr, br, bl] = r;
  const x = -w / 2;
  const y = -h / 2;
  const s = new THREE.Shape();
  s.moveTo(x + bl, y);
  s.lineTo(x + w - br, y);
  if (br) s.quadraticCurveTo(x + w, y, x + w, y + br);
  s.lineTo(x + w, y + h - tr);
  if (tr) s.quadraticCurveTo(x + w, y + h, x + w - tr, y + h);
  s.lineTo(x + tl, y + h);
  if (tl) s.quadraticCurveTo(x, y + h, x, y + h - tl);
  s.lineTo(x, y + bl);
  if (bl) s.quadraticCurveTo(x, y, x + bl, y);
  return s;
}

/** A device body: an extruded rounded rectangle, thickness d centred on z = 0, bevelled edges. */
function bodyGeometry(w: number, h: number, d: number, r: Radii, bevel: number) {
  const ins = (v: number) => Math.max(v - bevel, 0.0001);
  const shape = roundedRect(w - 2 * bevel, h - 2 * bevel, [ins(r[0]), ins(r[1]), ins(r[2]), ins(r[3])]);
  const depth = d - 2 * bevel;
  const g = new THREE.ExtrudeGeometry(shape, { depth, bevelEnabled: true, bevelThickness: bevel, bevelSize: bevel, bevelSegments: 5, curveSegments: 28 });
  g.translate(0, 0, -depth / 2);
  g.computeVertexNormals();
  return g;
}

/** A flat rounded screen, UVs mapped to the given part of its texture. */
function screenGeometry(w: number, h: number, r: Radii, uv: [number, number, number, number] = [0, 1, 0, 1]) {
  const g = new THREE.ShapeGeometry(roundedRect(w, h, r), 28);
  const pos = g.attributes.position;
  const uvs = new Float32Array(pos.count * 2);
  for (let i = 0; i < pos.count; i++) {
    const u = (pos.getX(i) + w / 2) / w;
    const v = (pos.getY(i) + h / 2) / h;
    uvs[i * 2] = uv[0] + u * (uv[1] - uv[0]);
    uvs[i * 2 + 1] = uv[2] + v * (uv[3] - uv[2]);
  }
  g.setAttribute("uv", new THREE.BufferAttribute(uvs, 2));
  return g;
}

/** requestAnimationFrame where it exists (the page, Chrome's and Firefox's workers), else a timer */
const raf = (cb: (t: number) => void) =>
  typeof requestAnimationFrame === "function" ? requestAnimationFrame(cb) : setTimeout(() => cb(performance.now()), 16);

/* ---------------------------------------------------------------- the lab */

export function createLab(canvas: HTMLCanvasElement | OffscreenCanvas, opts: LabOpts) {
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true, powerPreference: "high-performance" });
  renderer.setPixelRatio(Math.min(opts.dpr || 1, 2));
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.NeutralToneMapping;
  renderer.toneMappingExposure = 1.0;

  const scene = new THREE.Scene();
  const pmrem = new THREE.PMREMGenerator(renderer);
  let envTex: THREE.Texture | null = null; // built in start(), in its own idle slice

  const camera = new THREE.PerspectiveCamera(26, 1, 0.1, 200);
  camera.position.set(0, 1.4, 38);
  camera.lookAt(0, 0, 0);

  const key = new THREE.DirectionalLight(0xffffff, 1.1);
  key.position.set(-8, 12, 14);
  scene.add(key);

  /* materials, shared */
  const anisotropy = renderer.capabilities.getMaxAnisotropy();
  const glass = new THREE.MeshPhysicalMaterial({ color: 0x15181c, metalness: 0.1, roughness: 0.22, clearcoat: 1, clearcoatRoughness: 0.08 });
  const metal = new THREE.MeshPhysicalMaterial({ color: 0x8d949c, metalness: 1, roughness: 0.28 });
  const dark = new THREE.MeshStandardMaterial({ color: 0x0b0d10, metalness: 0.2, roughness: 0.4 });
  const lensGlass = new THREE.MeshPhysicalMaterial({ color: 0x050608, metalness: 0, roughness: 0.05, clearcoat: 1 });
  const textures = new Map<string, Promise<THREE.Texture>>();
  // ImageBitmaps decode off the main thread and work inside a worker (flipped at decode time)
  const loader = new THREE.ImageBitmapLoader().setOptions({ imageOrientation: "flipY" });
  const loadTex = (k: string) => {
    if (!textures.has(k)) {
      textures.set(
        k,
        loader.loadAsync(opts.textureBase + TEX[k]).then((bitmap) => {
          const t = new THREE.Texture(bitmap as ImageBitmap);
          t.flipY = false;
          t.needsUpdate = true;
          t.colorSpace = THREE.SRGBColorSpace;
          t.anisotropy = anisotropy;
          t.minFilter = THREE.LinearMipmapLinearFilter;
          t.generateMipmaps = true;
          return t;
        }),
      );
    }
    return textures.get(k)!;
  };
  // screens are unlit: the app's pixels show exactly as captured (no tone mapping, no glare)
  const screenMat = (t: THREE.Texture) => new THREE.MeshBasicMaterial({ map: t, toneMapped: false, polygonOffset: true, polygonOffsetFactor: -2 });

  /* scene graph: stage (yaw/pitch) > fit (scale) > device groups */
  const stage = new THREE.Group();
  const fit = new THREE.Group();
  stage.add(fit);
  scene.add(stage);

  // soft contact shadow under the devices
  const shadowTex = (() => {
    // OffscreenCanvas in the worker; a plain canvas on the page fallback (browsers without it)
    const c: OffscreenCanvas | HTMLCanvasElement =
      typeof OffscreenCanvas !== "undefined" ? new OffscreenCanvas(256, 256) : Object.assign(document.createElement("canvas"), { width: 256, height: 256 });
    const g = c.getContext("2d") as OffscreenCanvasRenderingContext2D | CanvasRenderingContext2D;
    const grd = g.createRadialGradient(128, 128, 0, 128, 128, 128);
    grd.addColorStop(0, "rgba(0,0,0,0.55)");
    grd.addColorStop(0.55, "rgba(0,0,0,0.16)");
    grd.addColorStop(1, "rgba(0,0,0,0)");
    g.fillStyle = grd;
    g.fillRect(0, 0, 256, 256);
    const t = new THREE.CanvasTexture(c);
    t.colorSpace = THREE.SRGBColorSpace;
    return t;
  })();
  const shadow = new THREE.Mesh(new THREE.PlaneGeometry(1, 1), new THREE.MeshBasicMaterial({ map: shadowTex, transparent: true, depthWrite: false }));
  shadow.rotation.x = -Math.PI / 2;
  shadow.position.y = -7.6;
  shadow.scale.set(13, 3.2, 1);
  scene.add(shadow);

  type Dev = {
    id: DeviceId;
    root: THREE.Group; // centring offset lives here
    size: [number, number]; // open footprint (w, h) for fitting
    hinge?: { pivot: THREE.Group; spine: THREE.Group; crease: THREE.Mesh; axis: "y" | "x"; span: number };
    built: boolean;
  };
  const devs = new Map<DeviceId, Dev>();

  /* -------- builders */

  function camera3(parent: THREE.Object3D, x: number, y: number, z: number, big: boolean) {
    // the camera module: well-rounded corners and a soft bevel on its raised edge
    const bw = big ? 2.3 : 1.9;
    const bh = big ? 3.4 : 2.0;
    const br = big ? 0.72 : 0.62;
    const bump = new THREE.Mesh(bodyGeometry(bw, bh, 0.2, [br, br, br, br], 0.07), [glass, glass]);
    bump.position.set(x, y, z - 0.08);
    parent.add(bump);
    const lenses = big ? [[-0.45, 0.85], [-0.45, -0.85], [0.6, 0]] : [[0, 0.45], [0, -0.45]];
    for (const [lx, ly] of lenses) {
      const ring = new THREE.Mesh(new THREE.CylinderGeometry(0.42, 0.42, 0.12, 40), metal);
      ring.rotation.x = Math.PI / 2;
      ring.position.set(x + lx, y + ly, z - 0.2);
      parent.add(ring);
      const lens = new THREE.Mesh(new THREE.CylinderGeometry(0.31, 0.31, 0.13, 40), lensGlass);
      lens.rotation.x = Math.PI / 2;
      lens.position.set(x + lx, y + ly, z - 0.205);
      parent.add(lens);
    }
  }

  function punchHole(parent: THREE.Object3D, x: number, y: number, z: number, r = 0.13) {
    const m = new THREE.Mesh(new THREE.CircleGeometry(r, 32), dark);
    m.position.set(x, y, z + 0.004);
    parent.add(m);
  }

  /** a slab phone or tablet: body, screen, punch-hole camera, back camera */
  async function slab(id: DeviceId, tex: string, sw: number, bezel: number, d: number, screenR: number, camBig: boolean) {
    const [pw, ph] = PX[tex];
    const sh = sw * (ph / pw);
    const w = sw + 2 * bezel;
    const h = sh + 2 * bezel;
    const R = screenR + bezel * 0.85;
    const root = new THREE.Group();
    root.add(new THREE.Mesh(bodyGeometry(w, h, d, [R, R, R, R], Math.min(0.14, d * 0.2)), [glass, metal]));
    const t = await loadTex(tex);
    const scr = new THREE.Mesh(screenGeometry(sw, sh, [screenR, screenR, screenR, screenR]), screenMat(t));
    scr.position.z = d / 2 + 0.002;
    root.add(scr);
    if (id === "tablet") punchHole(root, 0, sh / 2 + bezel / 2, d / 2, 0.12);
    else punchHole(root, 0, sh / 2 - 0.42, d / 2 + 0.002);
    camera3(root, id === "tablet" ? -w / 2 + 2.2 : -w / 2 + 1.75, h / 2 - (id === "tablet" ? 2.2 : 2.05), -d / 2, camBig);
    return { root, size: [w, h] as [number, number] };
  }

  /**
   * a two-panel foldable. axis "y": book fold, panels side by side (left fixed, right turns).
   * axis "x": flip, panels stacked (bottom fixed, top turns). The turning panel hangs in a pivot
   * on the front surface line, so the panels meet screen to screen when closed; a spine cylinder
   * of radius d turns at half the angle and covers the hinge from outside at every angle.
   */
  async function foldable(axis: "y" | "x") {
    const d = axis === "y" ? 0.58 : 0.7;
    const bevel = 0.09;
    const outerR = axis === "y" ? 1.15 : 1.25;
    const inner = axis === "y" ? 0.05 : 0.05;
    const root = new THREE.Group();

    if (axis === "y") {
      const [pw, ph] = PX.foldLeft;
      const sw = 6.3; // one panel's screen
      const sh = sw * (ph / pw);
      const bz = 0.24;
      const pw2 = sw + bz; // panel width: bezel on the outer side only
      const phh = sh + 2 * bz;
      const [tl, tr, tc] = await Promise.all([loadTex("foldLeft"), loadTex("foldRight"), loadTex("foldCover")]);
      // left (fixed) panel: x in [-pw2, 0]
      const left = new THREE.Group();
      const lb = new THREE.Mesh(bodyGeometry(pw2, phh, d, [outerR, inner, inner, outerR], bevel), [glass, metal]);
      lb.position.x = -pw2 / 2;
      left.add(lb);
      const ls = new THREE.Mesh(screenGeometry(sw, sh, [0.85, 0, 0, 0.85]), screenMat(tl));
      ls.position.set(-sw / 2, 0, d / 2 + 0.002);
      left.add(ls);
      camera3(left, -pw2 + 1.6, phh / 2 - 2.1, -d / 2, true);
      root.add(left);
      // right panel in the pivot (pivot on the front surface line at x = 0)
      const pivot = new THREE.Group();
      pivot.position.set(0, 0, d / 2);
      const right = new THREE.Group();
      right.position.z = -d / 2;
      const rb = new THREE.Mesh(bodyGeometry(pw2, phh, d, [inner, outerR, outerR, inner], bevel), [glass, metal]);
      rb.position.x = pw2 / 2;
      right.add(rb);
      const rs = new THREE.Mesh(screenGeometry(sw, sh, [0, 0.85, 0.85, 0]), screenMat(tr));
      rs.position.set(sw / 2, 0, d / 2 + 0.002);
      right.add(rs);
      punchHole(right, sw / 2, sh / 2 - 0.5, d / 2 + 0.002);
      // cover display on the back of the right panel
      const [cw0, ch0] = PX.foldCover;
      const ch = sh * 0.97;
      const cw = ch * (cw0 / ch0);
      const cover = new THREE.Mesh(screenGeometry(cw, ch, [0.75, 0.75, 0.75, 0.75]), screenMat(tc));
      cover.rotation.y = Math.PI;
      cover.position.set(pw2 / 2, 0, -d / 2 - 0.002);
      right.add(cover);
      punchHole(cover, 0, ch / 2 - 0.45, 0.002);
      pivot.add(right);
      root.add(pivot);
      // spine: a cylinder of radius d on the hinge axis, only its outward half
      const spine = new THREE.Group();
      spine.position.set(0, 0, d / 2);
      const cyl = new THREE.Mesh(new THREE.CylinderGeometry(d * 0.995, d * 0.995, phh - 2 * bevel, 48, 1, false, Math.PI / 2, Math.PI), metal);
      spine.add(cyl);
      root.add(spine);
      // the crease: a faint shade along the hinge, only when open flat
      const crease = new THREE.Mesh(
        new THREE.PlaneGeometry(0.22, sh),
        new THREE.MeshBasicMaterial({ color: 0x000000, transparent: true, opacity: 0.1, depthWrite: false, toneMapped: false }),
      );
      crease.position.z = d / 2 + 0.006;
      root.add(crease);
      return { root, size: [pw2 * 2, phh] as [number, number], hinge: { pivot, spine, crease, axis, span: pw2 } };
    }

    // flip: x axis, panels stacked
    const [pw, ph] = PX.flipInner;
    const sw = 6.5;
    const sh = sw * (ph / pw); // the whole inner screen
    const bz = 0.24;
    const w = sw + 2 * bz;
    const halfH = sh / 2 + bz;
    const [ti, tc] = await Promise.all([loadTex("flipInner"), loadTex("flipCover")]);
    const bottom = new THREE.Group();
    const bb = new THREE.Mesh(bodyGeometry(w, halfH, d, [inner, inner, outerR, outerR], bevel), [glass, metal]);
    bb.position.y = -halfH / 2;
    bottom.add(bb);
    const bs = new THREE.Mesh(screenGeometry(sw, sh / 2, [0, 0, 0.85, 0.85], [0, 1, 0, 0.5]), screenMat(ti));
    bs.position.set(0, -sh / 4, d / 2 + 0.002);
    bottom.add(bs);
    root.add(bottom);
    const pivot = new THREE.Group();
    pivot.position.set(0, 0, d / 2);
    const top = new THREE.Group();
    top.position.z = -d / 2;
    const tb = new THREE.Mesh(bodyGeometry(w, halfH, d, [outerR, outerR, inner, inner], bevel), [glass, metal]);
    tb.position.y = halfH / 2;
    top.add(tb);
    const ts = new THREE.Mesh(screenGeometry(sw, sh / 2, [0.85, 0.85, 0, 0], [0, 1, 0.5, 1]), screenMat(ti));
    ts.position.set(0, sh / 4, d / 2 + 0.002);
    top.add(ts);
    punchHole(top, 0, sh / 2 - 0.42, d / 2 + 0.002);
    // cover display on the back of the top half, with the cameras beside it
    const [cw0, ch0] = PX.flipCover;
    const cw = w - 2 * bz - 0.1;
    const chh = cw * (ch0 / cw0);
    const cover = new THREE.Mesh(screenGeometry(cw, chh, [0.7, 0.7, 0.7, 0.7]), screenMat(tc));
    // turned about x (not y): once the top half folds over, the cover reads upright. On the closed
    // device the hinge is at the top; the two rear cameras sit side by side ("owl eyes") between the
    // hinge and the display, and the display sits below them. Small y here = near the hinge.
    const camBand = 1.75;
    cover.rotation.x = Math.PI;
    cover.position.set(0, bz + camBand + chh / 2, -d / 2 - 0.002);
    top.add(cover);
    // the pair sits near the left edge (as seen on the closed device), like the other phones' modules;
    // the top half folds about x, so its local x stays the viewer's x
    const camX = -w / 2 + bz + 0.82;
    for (const lx of [camX, camX + 1.56]) {
      const ring = new THREE.Mesh(new THREE.CylinderGeometry(0.56, 0.56, 0.12, 48), metal);
      ring.rotation.x = Math.PI / 2;
      ring.position.set(lx, bz + camBand / 2 + 0.05, -d / 2 - 0.04);
      top.add(ring);
      const lens = new THREE.Mesh(new THREE.CylinderGeometry(0.43, 0.43, 0.13, 48), lensGlass);
      lens.rotation.x = Math.PI / 2;
      lens.position.set(lx, bz + camBand / 2 + 0.05, -d / 2 - 0.045);
      top.add(lens);
    }
    pivot.add(top);
    root.add(pivot);
    const spine = new THREE.Group();
    spine.position.set(0, 0, d / 2);
    const cyl = new THREE.Mesh(new THREE.CylinderGeometry(d * 0.995, d * 0.995, w - 2 * bevel, 48, 1, false, Math.PI / 2, Math.PI), metal);
    cyl.rotation.z = Math.PI / 2;
    spine.add(cyl);
    root.add(spine);
    const crease = new THREE.Mesh(
      new THREE.PlaneGeometry(sw, 0.22),
      new THREE.MeshBasicMaterial({ color: 0x000000, transparent: true, opacity: 0.1, depthWrite: false, toneMapped: false }),
    );
    crease.position.z = d / 2 + 0.006;
    root.add(crease);
    return { root, size: [w, halfH * 2] as [number, number], hinge: { pivot, spine, crease, axis, span: halfH } };
  }

  async function build(id: DeviceId): Promise<Dev> {
    const have = devs.get(id);
    if (have) return have;
    let made: { root: THREE.Group; size: [number, number]; hinge?: Dev["hinge"] };
    if (id === "standard") made = await slab("standard", "standard", 6.9, 0.24, 0.8, 0.95, true);
    else if (id === "compact") made = await slab("compact", "compact", 6.2, 0.24, 0.84, 0.9, false);
    else if (id === "tablet") made = await slab("tablet", "tablet", 21.6, 0.75, 0.62, 0.85, false);
    else made = await foldable(id === "fold" ? "y" : "x");
    const dev: Dev = { id, ...made, built: true };
    dev.root.visible = false;
    fit.add(dev.root);
    devs.set(id, dev);
    return dev;
  }

  /* -------- state */

  const state: LabState = { device: "standard", hinge: 180, back: false, demo: false };
  const view = { yaw: 0, pitch: 0, tYaw: 0, tPitch: 0 };
  const hinge: Record<"fold" | "flip", number> = { fold: 180, flip: 180 };
  let current: Dev | null = null;
  let size = { w: 1, h: 1 };
  let running = false;
  let visible = true;
  let disposed = false;
  let tweens = 0;
  let firstFrame = false;
  const emit = () => opts.onState({ ...state });

  // world size the devices are fitted into (keeps each profile readable at the stage size)
  // The phones (standard, compact, fold, flip) share one scale, so their real sizes compare
  // (the compact is visibly smaller); the tablet gets its own fit, being twice as wide.
  const PHONE_BOX: [number, number] = [14.2, 17.2]; // the open fold's width, the open flip's height
  function fitScale(dev: Dev) {
    const aspect = size.w / size.h;
    const vh = 2 * Math.tan(THREE.MathUtils.degToRad(camera.fov / 2)) * camera.position.z;
    const maxH = vh * 0.9;
    const maxW = vh * aspect * 0.84;
    const box = dev.id === "tablet" ? dev.size : PHONE_BOX;
    return Math.min(maxH / box[1], maxW / box[0]);
  }

  function applyHinge(dev: Dev, deg: number) {
    if (!dev.hinge) return;
    const a = THREE.MathUtils.degToRad(180 - deg); // 0 open flat, π closed
    const { pivot, spine, crease, axis, span } = dev.hinge;
    if (axis === "y") {
      pivot.rotation.y = -a;
      spine.rotation.y = -a / 2;
      dev.root.position.x = (span / 2) * (a / Math.PI); // keep the device centred as it closes
    } else {
      pivot.rotation.x = a;
      spine.rotation.x = a / 2;
      dev.root.position.y = (span / 2) * (a / Math.PI);
    }
    (crease.material as THREE.MeshBasicMaterial).opacity = deg > 170 ? 0.1 * ((deg - 170) / 10) : 0;
    crease.visible = deg > 170;
  }

  /* -------- render loop (on demand) */

  function frame() {
    if (disposed) return;
    running = false;
    const k = opts.reduced ? 1 : 0.14;
    view.yaw += (view.tYaw - view.yaw) * k;
    view.pitch += (view.tPitch - view.pitch) * k;
    stage.rotation.set(view.pitch, view.yaw, 0);
    renderer.render(scene, camera);
    if (!firstFrame) {
      firstFrame = true;
      opts.onFirstFrame();
    }
    const settling = Math.abs(view.tYaw - view.yaw) > 1e-4 || Math.abs(view.tPitch - view.pitch) > 1e-4;
    if ((settling || tweens > 0) && visible) invalidate();
  }
  function invalidate() {
    if (running || disposed || !visible) return;
    running = true;
    raf(frame);
  }
  const live = new Set<gsap.core.Tween>();
  const tween = (target: object, vars: gsap.TweenVars) => {
    tweens++;
    const t: gsap.core.Tween = gsap.to(target, {
      ...vars,
      onUpdate: () => {
        (vars.onUpdate as (() => void) | undefined)?.();
        invalidate();
      },
      onInterrupt: () => {
        tweens--;
      },
    });
    live.add(t);
    t.eventCallback("onComplete", () => {
      live.delete(t);
      tweens--;
      (vars.onComplete as (() => void) | undefined)?.();
      invalidate();
    });
    return t;
  };

  function resize(w: number, h: number) {
    size = { w: Math.max(1, w), h: Math.max(1, h) };
    renderer.setSize(size.w, size.h, false);
    camera.aspect = size.w / size.h;
    camera.updateProjectionMatrix();
    if (current) fit.scale.setScalar(fitScale(current));
    invalidate();
  }

  /* -------- public actions */

  // device changes run one after another, so quick clicks never overlap two cross-fades
  let queue: Promise<void> = Promise.resolve();
  let wanted: DeviceId = "standard";
  function show(id: DeviceId, animate = !opts.reduced) {
    wanted = id;
    queue = queue.then(() => (wanted === id ? swap(id, animate) : undefined));
    return queue;
  }
  async function swap(id: DeviceId, animate: boolean) {
    const prev = current;
    const next = await build(id);
    if (disposed) return;
    if (prev === next) return;
    state.device = id;
    state.back = false;
    if (next.hinge) {
      state.hinge = hinge[id as "fold" | "flip"];
      applyHinge(next, state.hinge);
    }
    const s = fitScale(next);
    if (!animate || !prev) {
      emit();
      if (prev) prev.root.visible = false;
      next.root.visible = true;
      fit.scale.setScalar(s);
      view.tYaw = view.yaw = prev ? -0.32 : 0;
      current = next;
      invalidate();
      return;
    }
    // switch with a small turn: the old device shrinks while the canvas fades out, the new one grows
    // in as it fades back. The fade is on the whole canvas (CSS opacity), never on the materials,
    // so a device is never see-through mid-switch.
    const o = { a: 1 };
    opts.onFade(0, 260, "cubic-bezier(0.55, 0.085, 0.68, 0.53)"); // power2.in
    await new Promise<void>((res) =>
      tween(o, {
        a: 0,
        duration: 0.26,
        ease: "power2.in",
        onUpdate: () => {
          prev.root.scale.setScalar(0.94 + 0.06 * o.a);
        },
        onComplete: res,
      }),
    );
    prev.root.visible = false;
    prev.root.scale.setScalar(1);
    current = next;
    emit(); // the label changes when the new device appears, not before
    fit.scale.setScalar(s);
    next.root.visible = true;
    next.root.scale.setScalar(0.94);
    view.yaw = view.yaw - 0.4;
    view.tYaw = -0.32;
    const n = { a: 0 };
    opts.onFade(1, 500, "cubic-bezier(0.215, 0.61, 0.355, 1)"); // power3.out
    await new Promise<void>((res) =>
      tween(n, {
        a: 1,
        duration: 0.5,
        ease: "power3.out",
        onUpdate: () => {
          next.root.scale.setScalar(0.94 + 0.06 * n.a);
        },
        onComplete: () => {
          opts.onFade(null);
          res();
        },
      }),
    );
  }

  function setHinge(deg: number, animate = false) {
    if (!current?.hinge) return;
    const id = current.id as "fold" | "flip";
    const to = THREE.MathUtils.clamp(deg, 0, 180);
    if (!animate || opts.reduced) {
      hinge[id] = to;
      state.hinge = to;
      applyHinge(current, to);
      emit();
      invalidate();
      return Promise.resolve();
    }
    const h = { v: hinge[id] };
    const dev = current;
    return new Promise<void>((res) =>
      tween(h, {
        v: to,
        duration: Math.max(0.5, (Math.abs(to - h.v) / 180) * 1.5),
        ease: "power2.inOut",
        onUpdate: () => {
          hinge[id] = h.v;
          state.hinge = Math.round(h.v);
          applyHinge(dev, h.v);
          emit();
        },
        onComplete: res,
      }),
    );
  }

  function turn(dYaw: number, dPitch: number) {
    view.tYaw += dYaw;
    view.tPitch = THREE.MathUtils.clamp(view.tPitch + dPitch, -0.5, 0.5);
    const facing = Math.cos(view.tYaw) < 0;
    if (facing !== state.back) {
      state.back = facing;
      emit();
    }
    invalidate();
  }

  function flipSide() {
    const back = !state.back;
    const base = Math.round(view.tYaw / (2 * Math.PI)) * 2 * Math.PI;
    view.tYaw = back ? base + Math.PI - 0.32 : base - 0.32;
    state.back = back;
    emit();
    invalidate();
  }

  function reset() {
    view.tYaw = -0.32;
    view.tPitch = 0.06;
    state.back = false;
    if (current?.hinge) void setHinge(180, true);
    emit();
    invalidate();
  }

  /* the one-time demo: a slow turn, every profile in turn, both hinges closing and opening */
  let demoStop = false;
  const wait = (s: number) => new Promise<void>((r) => gsap.delayedCall(s, r));
  async function demo() {
    if (opts.reduced) return;
    state.demo = true;
    emit();
    demoStop = false;
    const step = async (fn: () => Promise<unknown> | void, pause: number) => {
      if (demoStop || disposed) return false;
      await fn();
      if (demoStop || disposed) return false;
      await wait(pause);
      return !demoStop;
    };
    view.tYaw = -0.32;
    view.tPitch = 0.06;
    invalidate();
    // build the other devices one at a time in idle moments (no long main-thread task)
    void (async () => {
      for (const id of DEVICE_IDS) {
        if (disposed) return;
        await idle();
        const dev = await build(id);
        // upload its textures to the GPU now, not in the frame it first appears
        dev.root.traverse((o) => {
          const m = (o as THREE.Mesh).material as THREE.MeshBasicMaterial | undefined;
          if (m && !Array.isArray(m) && m.map) renderer.initTexture(m.map);
        });
      }
    })();
    const ok =
      (await step(() => wait(0.1), 1.6)) &&
      (await step(() => show("compact"), 1.8)) &&
      (await step(() => show("tablet"), 1.8)) &&
      (await step(() => show("fold"), 0.6)) &&
      (await step(() => setHinge(0, true), 0.5)) &&
      (await step(() => setHinge(180, true), 0.8)) &&
      (await step(() => show("flip"), 0.6)) &&
      (await step(() => setHinge(0, true), 0.5)) &&
      (await step(() => setHinge(180, true), 0.8)) &&
      (await step(() => show("standard"), 0));
    void ok;
    state.demo = false;
    emit();
  }
  function stopDemo() {
    if (!state.demo) return;
    demoStop = true;
    state.demo = false;
    emit();
  }

  function idle() {
    return new Promise<void>((r) => {
      const ric = (globalThis as { requestIdleCallback?: (cb: () => void, o?: { timeout: number }) => number }).requestIdleCallback;
      if (ric) ric(() => r(), { timeout: 600 });
      else setTimeout(r, 30);
    });
  }

  /* -------- visibility, context loss, disposal */

  /** the host reports whether the canvas is on screen and the tab visible */
  function setVisible(v: boolean) {
    visible = v;
    if (visible) invalidate();
  }
  const onLost = (e: Event) => {
    e.preventDefault();
    opts.onLost();
  };
  canvas.addEventListener("webglcontextlost", onLost);

  function dispose() {
    disposed = true;
    demoStop = true;
    canvas.removeEventListener("webglcontextlost", onLost);
    live.forEach((t) => t.kill());
    scene.traverse((o) => {
      const m = o as THREE.Mesh;
      if (m.geometry) m.geometry.dispose();
      const mats = m.material ? (Array.isArray(m.material) ? m.material : [m.material]) : [];
      mats.forEach((mat) => {
        (mat as THREE.MeshStandardMaterial).emissiveMap?.dispose();
        (mat as THREE.MeshBasicMaterial).map?.dispose();
        mat.dispose();
      });
    });
    envTex?.dispose();
    pmrem.dispose();
    renderer.dispose();
  }

  /* -------- start: the standard phone, front on, matching the static picture */
  async function start(w: number, h: number) {
    resize(w, h);
    // the work is split into idle slices so no single main-thread task runs long
    await idle();
    envTex = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
    scene.environment = envTex;
    await idle();
    const first = await build("standard");
    first.root.visible = true; // compileAsync only compiles what is visible
    await idle();
    await renderer.compileAsync(scene, camera); // shaders compile off the critical path where supported
    await show("standard", false);
    view.yaw = view.tYaw = 0;
    view.pitch = view.tPitch = 0;
    invalidate();
  }

  return { start, show, setHinge, turn, flipSide, reset, demo, stopDemo, dispose, invalidate, resize, setVisible };
}

export type Lab = ReturnType<typeof createLab>;
