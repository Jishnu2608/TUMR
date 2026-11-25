"use client";

import { useEffect, useRef, useState } from "react";
import { RotateCcw } from "lucide-react";
import { prefersReducedMotion } from "@/design-system/motion";
import type { SceneController } from "./particles";

/**
 * R9: "the agent is not the runtime". One scene in three beats:
 * 1. Claude Code acts: its real MCP calls dissolve into particles that travel agent -> Runtime Core
 *    -> phone and settle on the running app (the particle dissolve, D5);
 * 2. Claude Code leaves: its card scatters and greys out, while the Core and the app stay lit;
 * 3. Codex takes over the same session: its calls land on the same running app as a reload.
 *
 * The markup is the final state (Codex connected, Core and app running), so it reads with script
 * off and under reduced motion, and the three steps below the scene tell the story in words for
 * everyone. The particle engine (particles.ts) loads only when the section comes near, plays once
 * on entry, keeps its final state and can be replayed; no pin, no React state per frame.
 */

const CLAUDE_CALLS = ["runtime_status", 'run_project { "path": "C:\\\\Projects\\\\wayfarer" }', 'tap { "x": 540, "y": 1210 }', "screenshot"];
const CODEX_CALLS = ["runtime_status", 'reload { "kind": "hot" }', 'get_diagnostics { "category": "runtime" }'];

function Card({ who, calls, hidden }: { who: "claude" | "codex"; calls: string[]; hidden?: boolean }) {
  const name = who === "claude" ? "Claude Code" : "Codex";
  return (
    <div className={hidden ? "as-card as-card-off" : "as-card"} data-as-card={who}>
      <p className="as-card-head">
        <span className="as-dot" data-as-dot />
        <span data-as-label>{name}, connected</span>
      </p>
      <ul className="as-calls" data-as-lines>
        {calls.map((c) => (
          <li key={c}>
            <code>{c}</code>
          </li>
        ))}
      </ul>
    </div>
  );
}

export function AgentScene() {
  const root = useRef<HTMLDivElement>(null);
  const ctrl = useRef<SceneController | null>(null);
  const [canReplay, setCanReplay] = useState(false);

  useEffect(() => {
    const el = root.current;
    if (!el || prefersReducedMotion()) return;
    const probe = document.createElement("canvas");
    if (!probe.getContext("2d")) return;
    let cancelled = false;
    // load the engine only when the scene comes near (no particle code in the first page load)
    const io = new IntersectionObserver(
      async ([e]) => {
        if (!e.isIntersecting) return;
        io.disconnect();
        const { createScene } = await import("./particles");
        if (cancelled) return;
        ctrl.current = createScene(el, { onDone: () => setCanReplay(true) });
      },
      { rootMargin: "400px 0px" },
    );
    io.observe(el);
    return () => {
      cancelled = true;
      io.disconnect();
      ctrl.current?.dispose();
      ctrl.current = null;
    };
  }, []);

  return (
    <figure className="as" aria-labelledby="as-caption">
      <div className="as-stage" ref={root} data-as-stage>
        <canvas className="as-canvas" aria-hidden data-as-canvas />
        <div className="as-col as-col-agent">
          <p className="as-tag">Coding agent</p>
          <div className="as-cards">
            <Card who="claude" calls={CLAUDE_CALLS} hidden />
            <Card who="codex" calls={CODEX_CALLS} />
          </div>
        </div>

        <div className="as-col as-col-core">
          <p className="as-tag">Runtime Core</p>
          <div className="as-core" data-as-core>
            <span className="as-core-glow" aria-hidden data-as-glow />
            <span className="as-core-name">tumr</span>
            <span className="as-core-up" data-as-uptime>
              up 6:21:00
            </span>
          </div>
        </div>

        <div className="as-col as-col-phone">
          <p className="as-tag">Your app</p>
          <div className="as-phone">
            <div className="as-screen" data-as-screen>
              {/* eslint-disable-next-line @next/next/no-img-element -- static export, pre-sized WebP */}
              <img src="/images/device-lab/standard.webp" width={880} height={1880} alt="" loading="lazy" decoding="async" />
              <span className="as-flash" aria-hidden data-as-flash />
            </div>
          </div>
          <p className="as-status">
            <span className="as-run-dot" aria-hidden />
            <span data-as-status>Running, reloaded</span>
          </p>
        </div>
      </div>

      <figcaption id="as-caption" className="as-steps">
        <ol>
          <li data-as-step="1">
            <span className="as-step-n">1</span>
            Claude Code calls the runtime over MCP: it runs the app, taps and takes a screenshot.
          </li>
          <li data-as-step="2">
            <span className="as-step-n">2</span>
            Claude Code disconnects. The emulator and your app keep running.
          </li>
          <li data-as-step="3">
            <span className="as-step-n">3</span>
            Codex connects to the same session and reloads the app. Same session; nothing restarted.
          </li>
        </ol>
        <button type="button" className="as-replay" hidden={!canReplay} onClick={() => ctrl.current?.replay()}>
          <RotateCcw aria-hidden className="size-4" />
          Replay
        </button>
      </figcaption>
    </figure>
  );
}
