"use client";

import { useRef, type ReactNode } from "react";
import { gsap, ease, prefersReducedMotion, registerGsap, useGSAP } from "@/design-system/motion";

/**
 * R6: plays each [data-motif] inside it once, when it scrolls into view. The markup arrives from
 * the server in its final state; this sets each motif's start state only when motion is allowed,
 * and an anchor jump past a motif (onLeave) or a 2.5 s safety timer shows it finished, so nothing
 * stays blank. Each motif type has its own movement (see PillarMotifs).
 */
export function MotifPlayer({ children, className }: { children: ReactNode; className?: string }) {
  const root = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      const { ScrollTrigger } = registerGsap();
      if (prefersReducedMotion() || !root.current) return;
      const timers: number[] = [];

      for (const el of gsap.utils.toArray<HTMLElement>("[data-motif]", root.current)) {
        const tl = build(el);
        if (!tl) continue;
        // a finished timeline ignores play(), so each motif plays once whatever fires
        ScrollTrigger.create({
          trigger: el,
          start: "top 85%",
          onEnter: () => tl.play(),
          onEnterBack: () => tl.play(),
          onLeave: () => tl.progress(1), // jumped past it: show it finished
        });
        timers.push(
          window.setTimeout(() => {
            if (tl.progress() === 0 && el.getBoundingClientRect().top < window.innerHeight * 0.85) tl.play();
          }, 2500),
        );
      }
      return () => timers.forEach((t) => clearTimeout(t));
    },
    { scope: root },
  );

  return (
    <div ref={root} className={className}>
      {children}
    </div>
  );
}

function build(el: HTMLElement): gsap.core.Timeline | null {
  const q = gsap.utils.selector(el);
  const tl = gsap.timeline({ paused: true, delay: 0.25, defaults: { ease: ease.enter } });

  switch (el.dataset.motif) {
    case "tools":
      // staggered type: each component arrives, then its tick
      tl.fromTo(q("[data-motif-item]"), { opacity: 0, y: 8 }, { opacity: 1, y: 0, duration: 0.45, stagger: 0.12 }).fromTo(
        q("[data-motif-tick]"),
        { scale: 0, transformOrigin: "50% 50%" },
        { scale: 1, duration: 0.35, ease: "back.out(3)", stagger: 0.12 },
        0.2,
      );
      return tl;
    case "profiles":
      // line drawing: each outline draws itself, smallest to largest, then its name
      tl.fromTo(q("[data-motif-line]"), { strokeDashoffset: 1, strokeDasharray: 1 }, { strokeDashoffset: 0, duration: 0.7, ease: ease.change, stagger: 0.11 }).fromTo(
        q("[data-motif-label]"),
        { opacity: 0 },
        { opacity: 1, duration: 0.3, stagger: 0.08 },
        0.4,
      );
      return tl;
    case "dock":
      // clip reveals: the agent's pane opens, then the phone beside it; one tap lands on the screen
      tl.fromTo(q("[data-motif-pane]"), { clipPath: "inset(0 100% 0 0)" }, { clipPath: "inset(0 0% 0 0)", duration: 0.7, ease: ease.change, stagger: 0.25 })
        .fromTo(q("[data-motif-item]"), { opacity: 0 }, { opacity: 1, duration: 0.25, stagger: 0.12 }, 0.35)
        .fromTo(q("[data-motif-tap]"), { scale: 0.4, opacity: 0, transformOrigin: "50% 50%" }, { scale: 1, opacity: 1, duration: 0.45, ease: "back.out(2.5)" }, 1.05);
      return tl;
    case "problems":
      // counting list: rows slide in from the left edge, then the repeated crash counts up
      tl.fromTo(q("[data-motif-item]"), { clipPath: "inset(0 100% 0 0)", x: -6 }, { clipPath: "inset(0 0% 0 0)", x: 0, duration: 0.55, ease: ease.change, stagger: 0.15 });
      for (const c of q("[data-motif-count]")) {
        const final = Number((c.textContent ?? "").replace(/\D/g, "")) || 1;
        const n = { v: 1 };
        tl.fromTo(c, { opacity: 0 }, { opacity: 1, duration: 0.2 }, 0.75).to(
          n,
          {
            v: final,
            duration: 0.6,
            ease: "steps(" + Math.max(1, final - 1) + ")",
            onUpdate: () => {
              c.textContent = `×${Math.round(n.v)}`;
            },
          },
          0.8,
        );
      }
      return tl;
    default:
      return null;
  }
}
