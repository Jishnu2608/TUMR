"use client";

import { useRef, type ReactNode } from "react";
import { gsap, registerGsap, useGSAP, prefersReducedMotion, ease, refreshAfterLayout } from "@/design-system/motion";

/**
 * Reveals `[data-reveal]` children once, as they enter the viewport (ScrollTrigger.batch, once).
 * No scrub, no pin: scrolling stays the reader's. Content is visible without script, and with
 * reduced motion nothing moves. useGSAP reverts every tween and trigger on unmount.
 *
 * R2 variants, chosen per element with `data-reveal="…"`:
 *   (empty) or "rise"  fade up 14 px — the default, unchanged from before R2
 *   "mask"   rises out of a mask (clip from the bottom), for headings and figures
 *   "clip-x" wipes in from the left, for images and panels
 *   "line"   draws from the left (scaleX), for rules, bars and underlines
 *   "type"   words rise one after another (SplitText, loaded only when used), for short headings
 *
 * Never blank (Scrolltide reveal rule): an element the reader scrolled past without it entering
 * (anchor jump, fast scroll) is shown on leave, and a safety timer shows anything still hidden
 * 2.5 s after it should have played.
 */
type Variant = "rise" | "mask" | "clip-x" | "line" | "type";

const FROM: Record<Exclude<Variant, "type">, gsap.TweenVars> = {
  rise: { opacity: 0, y: 14 },
  mask: { opacity: 0, y: 10, clipPath: "inset(0% 0% 100% 0%)" },
  "clip-x": { clipPath: "inset(0% 100% 0% 0%)" },
  line: { scaleX: 0, transformOrigin: "0% 50%" },
};
const TO: Record<Exclude<Variant, "type">, gsap.TweenVars> = {
  rise: { opacity: 1, y: 0, duration: 0.55 },
  mask: { opacity: 1, y: 0, clipPath: "inset(0% 0% 0% 0%)", duration: 0.7 },
  "clip-x": { clipPath: "inset(0% 0% 0% 0%)", duration: 0.8 },
  line: { scaleX: 1, duration: 0.7 },
};

function variantOf(el: HTMLElement): Variant {
  const v = el.dataset.reveal;
  return v === "mask" || v === "clip-x" || v === "line" || v === "type" ? v : "rise";
}

export function Reveal({ children, className }: { children: ReactNode; className?: string }) {
  const root = useRef<HTMLDivElement>(null);

  useGSAP(
    (_ctx, contextSafe) => {
      const { ScrollTrigger } = registerGsap();
      if (prefersReducedMotion() || !root.current) return;
      const items = gsap.utils.toArray<HTMLElement>("[data-reveal]", root.current);
      if (items.length === 0) return;

      const groups = new Map<Variant, HTMLElement[]>();
      for (const el of items) {
        const v = variantOf(el);
        groups.set(v, [...(groups.get(v) ?? []), el]);
      }

      const done = new WeakSet<HTMLElement>();
      const timers: number[] = [];

      for (const [variant, els] of groups) {
        if (variant === "type") continue;
        gsap.set(els, FROM[variant]);
        const show = (batch: Element[]) => {
          const fresh = (batch as HTMLElement[]).filter((e) => !done.has(e));
          if (fresh.length === 0) return;
          fresh.forEach((e) => done.add(e));
          // clearProps only when there is something to clear: an undefined value throws in GSAP
          gsap.to(fresh, {
            ...TO[variant],
            ease: ease.enter,
            stagger: 0.07,
            overwrite: true,
            ...(variant === "rise" ? {} : { clearProps: "clipPath,transformOrigin" }),
          });
        };
        ScrollTrigger.batch(els, { start: "top 88%", once: true, onEnter: show, onLeave: show });
      }

      // "type": split into words only when used (the plugin is not in the main bundle).
      const typed = groups.get("type");
      if (typed?.length) {
        gsap.set(typed, { opacity: 0 });
        import("gsap/SplitText").then(
          contextSafe!(({ SplitText }: typeof import("gsap/SplitText")) => {
            gsap.registerPlugin(SplitText);
            for (const el of typed) {
              const split = SplitText.create(el, { type: "words", mask: "words", aria: "auto" });
              gsap.set(el, { opacity: 1 });
              gsap.set(split.words, { yPercent: 110 });
              const play = () => {
                if (done.has(el)) return;
                done.add(el);
                gsap.to(split.words, { yPercent: 0, duration: 0.7, ease: ease.enter, stagger: 0.05, onComplete: () => split.revert() });
              };
              ScrollTrigger.create({ trigger: el, start: "top 88%", once: true, onEnter: play, onLeave: play });
            }
            ScrollTrigger.refresh();
          }),
          () => gsap.set(typed, { opacity: 1 }),
        );
      }

      // Safety net: anything above the fold line that is still hidden after 2.5 s is shown.
      const safety = contextSafe!(() => {
        const vh = window.innerHeight;
        const late = items.filter((e) => !done.has(e) && e.getBoundingClientRect().top < vh * 0.88);
        if (late.length) {
          late.forEach((e) => done.add(e));
          gsap.set(late, { opacity: 1, y: 0, yPercent: 0, scaleX: 1, clearProps: "clipPath,transformOrigin" });
        }
      });
      const onScrollEnd = () => {
        timers.push(window.setTimeout(safety, 2500));
      };
      window.addEventListener("scrollend", onScrollEnd);
      // keyboard: whatever receives focus inside is shown at once, never focused while invisible
      const onFocus = (e: FocusEvent) => {
        const hit = items.filter((el) => el.contains(e.target as Node) && Number(gsap.getProperty(el, "opacity")) < 1);
        if (!hit.length) return;
        hit.forEach((el) => done.add(el));
        gsap.killTweensOf(hit); // a reveal that started with the focus scroll must not keep it faint
        gsap.set(hit, { opacity: 1, y: 0, yPercent: 0, scaleX: 1, clearProps: "clipPath,transformOrigin" });
      };
      root.current.addEventListener("focusin", onFocus);
      const rootEl = root.current;
      timers.push(window.setTimeout(safety, 2500));
      refreshAfterLayout();

      return () => {
        window.removeEventListener("scrollend", onScrollEnd);
        rootEl.removeEventListener("focusin", onFocus);
        timers.forEach((t) => clearTimeout(t));
      };
    },
    { scope: root },
  );

  return (
    <div ref={root} className={className}>
      {children}
    </div>
  );
}
