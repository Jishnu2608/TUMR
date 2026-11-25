"use client";

import { useRef, useState, type KeyboardEvent } from "react";
import Link from "next/link";
import { gsap, ease, prefersReducedMotion, registerGsap, useGSAP } from "@/design-system/motion";
import { cn } from "@/lib/cn";
import { facts, frameworks } from "./frameworks-data";

/** Text with `code` spans (commands, files) set in the mono face. */
function Rich({ text }: { text: string }) {
  return text.split("`").map((part, i) => (i % 2 ? <code key={i}>{part}</code> : part));
}

/**
 * R7: one tab per framework, one panel of plain facts each. Every panel is in the HTML; the
 * inactive ones are display:none (a class, since Tailwind's preflight makes [hidden] !important),
 * and without script a <noscript> style shows all five stacked
 * (and hides the tab row), so the section reads fully with JS off. Selection: the panel's rows
 * settle in one after another (transform and opacity only); reduced motion swaps at once.
 * Arrow keys, Home and End move between tabs (the WAI-ARIA tabs pattern).
 */
export function FrameworkTabs() {
  const [active, setActive] = useState(0);
  const changed = useRef(false);
  const root = useRef<HTMLDivElement>(null);
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);

  useGSAP(
    () => {
      if (!changed.current || prefersReducedMotion() || !root.current) return;
      registerGsap();
      const panel = root.current.querySelector(`#fw-panel-${frameworks[active].id}`);
      if (!panel) return;
      gsap.fromTo(panel.querySelectorAll("[data-fw-row]"), { opacity: 0, y: 6 }, { opacity: 1, y: 0, duration: 0.35, stagger: 0.04, ease: ease.enter });
    },
    { scope: root, dependencies: [active], revertOnUpdate: true },
  );

  const select = (i: number) => {
    changed.current = true;
    setActive(i);
  };

  function onKey(e: KeyboardEvent<HTMLButtonElement>, i: number) {
    const last = frameworks.length - 1;
    const next = e.key === "ArrowRight" || e.key === "ArrowDown" ? (i === last ? 0 : i + 1) : e.key === "ArrowLeft" || e.key === "ArrowUp" ? (i === 0 ? last : i - 1) : e.key === "Home" ? 0 : e.key === "End" ? last : -1;
    if (next >= 0) {
      e.preventDefault();
      select(next);
      tabs.current[next]?.focus();
    }
  }

  return (
    <div ref={root} className="fw">
      <noscript>
        <style>{`.fw-tabs{display:none}.fw-panel-off{display:grid}`}</style>
      </noscript>
      <div role="tablist" aria-label="Frameworks" className="fw-tabs">
        {frameworks.map((f, i) => {
          const on = i === active;
          return (
            <button
              key={f.id}
              ref={(el) => {
                tabs.current[i] = el;
              }}
              type="button"
              role="tab"
              id={`fw-tab-${f.id}`}
              aria-selected={on}
              aria-controls={`fw-panel-${f.id}`}
              tabIndex={on ? 0 : -1}
              onClick={() => select(i)}
              onKeyDown={(e) => onKey(e, i)}
              className={cn("fw-tab", on && "fw-tab-on")}
            >
              <span className="fw-tab-name">{f.name}</span>
              <span className="fw-tab-lang">{f.toolchain}</span>
            </button>
          );
        })}
      </div>

      {frameworks.map((f, i) => (
        <div
          key={f.id}
          id={`fw-panel-${f.id}`}
          role="tabpanel"
          aria-labelledby={`fw-tab-${f.id}`}
          tabIndex={0}
          className={cn("fw-panel", i !== active && "fw-panel-off")}
        >
          <h3 className="fw-panel-title">{f.name}</h3>
          <dl className="fw-facts">
            {facts.map(({ key, label }) => (
              <div key={key} className="fw-row" data-fw-row>
                <dt>{label}</dt>
                <dd>
                  <Rich text={f[key] as string} />
                  {key === "save" && f.saveFigure && <span className="fw-figure">{f.saveFigure}</span>}
                </dd>
              </div>
            ))}
          </dl>
          <p className="fw-guide" data-fw-row>
            <Link className="link" href={f.guide.href}>
              {f.guide.label}
            </Link>
          </p>
        </div>
      ))}
    </div>
  );
}
