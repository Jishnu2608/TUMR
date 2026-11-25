"use client";

/**
 * Motion conventions (directive §5; R2 motion foundation, 2026-11-25). GSAP runs only in client
 * components through useGSAP, scoped to a ref, so every tween and ScrollTrigger is reverted on
 * unmount.
 *
 * - Content is in the HTML and visible without script; animations start from a state GSAP sets.
 * - Durations: fast 0.18 s (hover / press), base 0.45 s (reveals), slow 0.8 s (hero, diagrams).
 * - Eases: "power3.out" for entrances, "power2.inOut" for state changes, "none" for scrubbed lines.
 * - Under prefers-reduced-motion nothing moves: callers get `reduced === true` and set end states.
 * - Reveals play once and never leave a blank page: an element scrolled past (anchor jump, fast
 *   scroll) is shown too, and a safety timer shows anything still hidden.
 *
 * Pinning rules (owner constraints §4.1 and D5/D6 — read before adding any `pin`):
 * 1. At most ONE pinned section on a page, and never in the hero. The workflow tabs (D6) and the
 *    particle dissolve (D5) are not pinned. The only candidate today is the Device Lab (R8),
 *    and only if its static fallback works without the pin.
 * 2. Create ScrollTriggers top to bottom. A trigger created out of page order (lazy section,
 *    late data) must set `refreshPriority` = its section's order from `SECTION_ORDER`, so pins
 *    and the triggers below them are measured in page order.
 * 3. Anchor links must land on their heading: `html { scroll-padding-top }` covers the sticky
 *    header; after fonts or images change the layout call `refreshAfterLayout()`. A pin adds
 *    height, so a pinned section keeps `pinSpacing: true` (never false) and anchors below it
 *    still land correctly.
 * 4. No scroll-jacking: no smooth-scroll library, no snapping, no `scrub` on text. Scrub is only
 *    for drawn lines / diagrams that track the scrollbar, with `ease: "none"`.
 * 5. Pinned and scrubbed work is off under reduced motion and below 768 px (use `useMedia`).
 * 6. Scenes (canvas / WebGL) pause when off screen (`useInView`) or the tab is hidden
 *    (`usePageVisible`).
 */
import { gsap } from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { useGSAP } from "@gsap/react";

let registered = false;
export function registerGsap() {
  if (!registered && typeof window !== "undefined") {
    gsap.registerPlugin(useGSAP, ScrollTrigger);
    registered = true;
    if (process.env.NODE_ENV !== "production") {
      // R2 acceptance: no duplicate ScrollTriggers. Logged in development only.
      ScrollTrigger.addEventListener("refresh", () => {
        console.debug(`[motion] ScrollTriggers: ${ScrollTrigger.getAll().length}`);
      });
      (window as unknown as { __motionTriggers?: () => number }).__motionTriggers = () => ScrollTrigger.getAll().length;
    }
  }
  return { gsap, ScrollTrigger };
}

export const dur = { fast: 0.18, base: 0.45, slow: 0.8 } as const;
export const ease = { enter: "power3.out", change: "power2.inOut", line: "none" } as const;

/** Breakpoints and preferences for `gsap.matchMedia` (used through `useMedia`). */
export const media = {
  desktop: "(min-width: 1024px)",
  tablet: "(min-width: 768px) and (max-width: 1023.98px)",
  mobile: "(max-width: 767.98px)",
  motion: "(prefers-reduced-motion: no-preference)",
  reduced: "(prefers-reduced-motion: reduce)",
} as const;
export type MediaKey = keyof typeof media;

/** Homepage section order, for `refreshPriority` on triggers created out of page order (lower = higher on the page). */
export const SECTION_ORDER = [
  "hero",
  "pillars",
  "how",
  "frameworks",
  "lab",
  "agents",
  "vscode",
  "arch",
  "privacy",
  "oss",
  "faq",
  "close",
] as const;
export function refreshPriority(section: (typeof SECTION_ORDER)[number]) {
  return -SECTION_ORDER.indexOf(section);
}

export function prefersReducedMotion() {
  return typeof window !== "undefined" && window.matchMedia(media.reduced).matches;
}

/**
 * Re-measure every trigger once fonts are ready (layout can move when web fonts swap in).
 * Shared: many sections may ask, the page refreshes once (each refresh measures every trigger).
 * Nothing to do when the fonts were already loaded before the triggers were created.
 */
let fontRefresh: Promise<void> | null = null;
export function refreshAfterLayout() {
  if (typeof window === "undefined" || fontRefresh) return;
  if (!document.fonts || document.fonts.status === "loaded") {
    fontRefresh = Promise.resolve();
    return;
  }
  fontRefresh = document.fonts.ready.then(() => {
    requestAnimationFrame(() => ScrollTrigger.refresh());
  });
}

/**
 * Reads design tokens (CSS custom properties) as resolved for the current theme, e.g.
 * `readTokens(["--signal", "--ink"])` → `{ "--signal": "#4263eb", "--ink": "#20252b" }`.
 * Canvas / WebGL scenes use this, since they cannot read CSS variables themselves.
 */
export function readTokens<T extends string>(names: readonly T[], el?: Element): Record<T, string> {
  const out = {} as Record<T, string>;
  if (typeof window === "undefined") return out;
  const cs = getComputedStyle(el ?? document.documentElement);
  for (const n of names) out[n] = cs.getPropertyValue(n).trim();
  return out;
}

/**
 * Calls `fn` whenever the theme changes: the `data-theme` switch, or the system scheme while no
 * explicit theme is set. Returns an unsubscribe function.
 */
export function onThemeChange(fn: () => void): () => void {
  if (typeof window === "undefined") return () => {};
  const mo = new MutationObserver(fn);
  mo.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  mq.addEventListener("change", fn);
  return () => {
    mo.disconnect();
    mq.removeEventListener("change", fn);
  };
}

export { gsap, ScrollTrigger, useGSAP };
