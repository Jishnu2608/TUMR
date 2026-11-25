"use client";

import { useRef } from "react";
import { gsap, registerGsap, useGSAP, prefersReducedMotion, ease } from "@/design-system/motion";

const clients = [
  { name: "tumr", note: "command line" },
  { name: "Desktop app", note: "wizard and preview" },
  { name: "tumr mcp", note: "MCP server for agents" },
];
const modules = [
  { name: "Environment and installer", note: "scan, plan, verified downloads" },
  { name: "Devices and sessions", note: "own adb server, emulator, snapshots" },
  { name: "Framework adapters", note: "Gradle, Flutter, React Native / Expo" },
  { name: "Diagnostics", note: "six categories, dedupe, sanitiser" },
  { name: "Preview stream", note: "emulator frames to the window" },
  { name: "Agent permissions", note: "scopes, approvals, audit log" },
];
const tools = ["JDK", "Android SDK and emulator", "Flutter SDK", "Node.js and Metro", "Gradle (project wrapper)"];

/**
 * Architecture as layers: clients → the user-only named pipe → Runtime Core → the official tools.
 * The layers are HTML (readable at every width, without script, under reduced motion); R10 adds
 * connectors between them (plain CSS lines, grown with transforms) that draw once when the diagram
 * scrolls into view: clients into the pipe, the pipe into the Core, the Core out to each tool. No
 * continuous motion.
 */
export function Architecture() {
  const root = useRef<HTMLDivElement>(null);

  useGSAP(
    () => {
      registerGsap();
      if (prefersReducedMotion()) return;
      const tl = gsap.timeline({
        defaults: { ease: ease.enter },
        scrollTrigger: { trigger: root.current, start: "top 75%", once: true },
      });
      // Only the connector lines that are shown at this width. On phones the buses are
      // display:none, and a transform tween on an unrendered element makes GSAP measure it by
      // moving it into the document and back, which cost seconds of main thread (WEB-017).
      const grow = (sel: string, vars: gsap.TweenVars, pos: string) => {
        const shown = Array.from(root.current!.querySelectorAll<HTMLElement>(`${sel} [data-arch-line]`)).filter(
          (el) => el.getClientRects().length > 0,
        );
        if (shown.length) tl.from(shown, { scale: 0, ease: ease.change, ...vars }, pos);
      };
      tl.from("[data-arch='client']", { opacity: 0, y: 16, duration: 0.5, stagger: 0.08 });
      grow(".arch-bus-in", { duration: 0.35, stagger: 0.1 }, "-=0.15");
      tl.from("[data-arch='pipe']", { opacity: 0, duration: 0.4 }, "-=0.1");
      grow(".arch-bus-core", { duration: 0.3 }, "-=0.1");
      tl.from("[data-arch='module']", { opacity: 0, y: 12, duration: 0.45, stagger: 0.06 }, "-=0.1");
      grow(".arch-bus-out", { duration: 0.3, stagger: 0.06 }, "-=0.2");
      tl.from("[data-arch='tool']", { opacity: 0, y: 6, duration: 0.4, stagger: 0.05 }, "-=0.3");
    },
    { scope: root },
  );

  return (
    <div ref={root} className="arch">
      <ol className="grid gap-3 sm:grid-cols-3" aria-label="Clients">
        {clients.map((c) => (
          <li key={c.name} data-arch="client" className="rounded-md border border-line bg-surface px-5 py-4">
            <p className="font-mono text-[0.95rem] font-semibold text-ink">{c.name}</p>
            <p className="t-desc mt-1">{c.note}</p>
          </li>
        ))}
      </ol>

      {/* the three clients join one line into the pipe (wide screens) */}
      <div className="arch-bus arch-bus-in" aria-hidden>
        {[1, 3, 5].map((n) => (
          <span key={n} className="arch-v arch-v-top" style={{ left: `${(n / 6) * 100}%` }} data-arch-line="v" />
        ))}
        <span className="arch-h" style={{ left: `${100 / 6}%`, right: `${100 / 6}%` }} data-arch-line="h" />
        <span className="arch-v arch-v-bottom" style={{ left: "50%" }} data-arch-line="v" />
      </div>

      <p className="arch-pipe" data-arch="pipe">
        <span>named pipe</span>
        <span>your Windows user only</span>
        <span>token per client</span>
      </p>

      <div className="arch-bus arch-bus-core" aria-hidden>
        <span className="arch-v arch-v-full" style={{ left: "50%" }} data-arch-line="v" />
      </div>

      <div className="rounded-lg bg-band p-5 text-band-ink sm:p-7 dark:bg-band dark:ring-1 dark:ring-band-line">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <p className="text-lg font-semibold tracking-tight">Runtime Core</p>
          <p className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-band-ink-2">
            <span className="font-mono">tumr-core</span>
            <span>one per Windows user</span>
            <span>keeps sessions alive</span>
          </p>
        </div>
        <ul className="mt-5 grid gap-px overflow-hidden rounded-md bg-band-line sm:grid-cols-2 lg:grid-cols-3">
          {modules.map((m) => (
            <li key={m.name} data-arch="module" className="bg-band-surface px-4 py-3 dark:bg-band-sunken">
              <p className="font-medium">{m.name}</p>
              <p className="mt-0.5 text-sm text-band-ink-2">{m.note}</p>
            </li>
          ))}
        </ul>
      </div>

      {/* the Core fans out to each official tool (wide screens) */}
      <div className="arch-bus arch-bus-out" aria-hidden>
        <span className="arch-v arch-v-top" style={{ left: "50%" }} data-arch-line="v" />
        <span className="arch-h" style={{ left: "10%", right: "10%" }} data-arch-line="h" />
        {[10, 30, 50, 70, 90].map((x) => (
          <span key={x} className="arch-v arch-v-bottom" style={{ left: `${x}%` }} data-arch-line="v" />
        ))}
      </div>

      <div className="arch-tools">
        <p className="t-label">Official tools it runs</p>
        <ul className="arch-tool-list">
          {tools.map((t) => (
            <li key={t} data-arch="tool" className="badge">
              {t}
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
