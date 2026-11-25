"use client";

import { useRef, type ReactNode } from "react";
import Link from "next/link";
import { ArrowRight, BookOpen } from "lucide-react";
import { gsap } from "@/design-system/motion";
import { useMedia } from "@/components/motion/hooks";

/**
 * Hero: eyebrow, headline, one paragraph and the two actions on the left; the visual on the right
 * (below the actions on phones). Since R8 the visual is the 3D device lab (DeviceLab), which shows
 * its static picture first and takes over once the page has loaded; the R5b showcase cycle is kept
 * in showcase-timeline.ts, not used here.
 *
 * The visual arrives as a prop from the (server) page. Desktop scroll handoff: the visual eases
 * back a little as the hero leaves (no pin). The headline never animates, so it is the LCP element.
 */
export function Hero({ hasRelease, showcase }: { hasRelease: boolean; showcase: ReactNode }) {
  const root = useRef<HTMLElement>(null);

  useMedia(
    (m) => {
      const el = root.current;
      if (!el || m.reduced || !m.motion || !m.desktop) return;
      gsap.to(el.querySelector("[data-hero-visual]"), {
        scale: 0.95,
        yPercent: -3,
        ease: "none",
        scrollTrigger: { trigger: el, start: "top top", end: "bottom top", scrub: 0.6 },
      });
    },
    root,
  );

  return (
    <section ref={root} aria-labelledby="hero-title" className="hero relative overflow-hidden">
      {/* flat tonal field behind the hero, no orb */}
      <div
        aria-hidden
        className="pointer-events-none absolute inset-x-0 top-0 -z-10 h-full bg-[radial-gradient(55%_60%_at_76%_45%,var(--glow),transparent_70%),linear-gradient(180deg,var(--bg-alt)_0%,var(--bg)_80%)]"
      />
      <div className="wrap grid items-center gap-12 pt-12 pb-16 md:pt-16 lg:min-h-[calc(100svh-var(--header-h))] lg:grid-cols-12 lg:gap-8 lg:py-12">
        <div className="min-w-0 lg:col-span-6">
          <p className="hero-eyebrow">
            Android runtime for AI coding agents
            <span className="whitespace-nowrap">
              <span aria-hidden className="hero-eyebrow-sep">
                ·
              </span>
              Windows 11
            </span>
          </p>
          <h1 id="hero-title" className="hero-title mt-5">
            Your agent writes the code. TUMR brings it to life.
          </h1>
          <p className="t-lead mt-6 max-w-[34rem]">
            A persistent Android development runtime that connects your coding agent to Android emulators, build tools and
            application diagnostics.
          </p>
          <div className="mt-9 flex flex-wrap items-center gap-3">
            <Link href="/download/" className="btn btn-primary btn-lg">
              {hasRelease ? "Download for Windows" : "See downloads"}
              <ArrowRight aria-hidden className="btn-icon size-4" />
            </Link>
            <Link href="/docs/getting-started/" className="btn btn-secondary btn-lg">
              <BookOpen aria-hidden className="size-4" />
              Read the getting-started guide
            </Link>
          </div>
        </div>
        <div className="min-w-0 lg:col-span-6">
          {showcase}
        </div>
      </div>
    </section>
  );
}
