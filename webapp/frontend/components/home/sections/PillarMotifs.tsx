import { Check } from "lucide-react";

/**
 * R6: one small motif per capability, each moving differently once it scrolls into view
 * (MotifPlayer): staggered type, line drawing, clip reveals, a counting list. Decorative
 * (aria-hidden): the row's text says everything. Server-rendered in its final state, so it reads
 * with script off and with reduced motion.
 */

/** 1. Toolchain: the components arrive one by one and get their tick (staggered type). */
export function ToolchainMotif() {
  return (
    <ul className="motif motif-tools" data-motif="tools" aria-hidden>
      {["JDK", "Android SDK", "Emulator", "Flutter", "Node.js"].map((t) => (
        <li key={t} data-motif-item>
          <Check className="motif-tick" data-motif-tick strokeWidth={2.5} />
          {t}
        </li>
      ))}
    </ul>
  );
}

/** 2. Emulator: the six device profiles drawn as outlines, smallest to largest (line drawing). */
export function ProfilesMotif() {
  // outlines on one baseline (y = 70), sizes in the motif's own units
  const shapes: { name: string; x: number; w: number; h: number; hinge?: "v" | "h" }[] = [
    { name: "compact", x: 10, w: 26, h: 50 },
    { name: "standard", x: 60, w: 29, h: 60 },
    { name: "large", x: 112, w: 32, h: 66 },
    { name: "fold", x: 162, w: 52, h: 60, hinge: "v" },
    { name: "flip", x: 232, w: 29, h: 60, hinge: "h" },
    { name: "tablet", x: 276, w: 84, h: 58 },
  ];
  return (
    <svg className="motif motif-profiles" viewBox="0 0 364 88" data-motif="profiles" aria-hidden>
      {shapes.map((s) => {
        const y = 70 - s.h;
        return (
          <g key={s.name}>
            <rect x={s.x + 0.75} y={y} width={s.w - 1.5} height={s.h} rx={5} pathLength={1} data-motif-line />
            {s.hinge === "v" && <line x1={s.x + s.w / 2} y1={y + 3} x2={s.x + s.w / 2} y2={70 - 3} pathLength={1} data-motif-line className="motif-hinge" />}
            {s.hinge === "h" && <line x1={s.x + 3} y1={y + s.h / 2} x2={s.x + s.w - 3} y2={y + s.h / 2} pathLength={1} data-motif-line className="motif-hinge" />}
            <text x={s.x + s.w / 2} y={84} textAnchor="middle" data-motif-label>
              {s.name}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

/** 3. Your app beside your agent: the agent's calls and the phone open side by side (clip reveals). */
export function DockMotif() {
  return (
    <div className="motif motif-dock" data-motif="dock" aria-hidden>
      <div className="motif-dock-calls" data-motif-pane>
        {["runtime_status", "screenshot", "tap", "reload"].map((c) => (
          <span key={c} data-motif-item>
            {c}
          </span>
        ))}
      </div>
      <div className="motif-dock-phone" data-motif-pane>
        <span className="motif-dock-screen">
          <i />
          <i />
          <i />
          <i className="motif-dock-hit" data-motif-tap />
        </span>
      </div>
    </div>
  );
}

/** 4. Problems: classified rows slide in, then the repeated crash shows its count (a counting list). */
export function ProblemsMotif() {
  const rows = [
    { cat: "Build", where: "MainActivity.kt:42", origin: "your code" },
    { cat: "Crash", where: "lib/main.dart:67", origin: "your code", count: 3 },
    { cat: "Environment", where: "Android SDK platform", origin: "environment" },
  ];
  return (
    <ul className="motif motif-problems" data-motif="problems" aria-hidden>
      {rows.map((r) => (
        <li key={r.where} data-motif-item>
          <span className={r.origin === "environment" ? "motif-cat motif-cat-env" : "motif-cat"}>{r.cat}</span>
          <code>{r.where}</code>
          {r.count ? (
            <span className="motif-count" data-motif-count>
              ×{r.count}
            </span>
          ) : (
            <span />
          )}
        </li>
      ))}
    </ul>
  );
}
