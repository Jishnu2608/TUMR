import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { Reveal } from "@/components/motion/Reveal";
import { AgentScene } from "../agents/AgentScene";

type ToolLite = { name: string; annotations: { readOnlyHint: boolean } };

/**
 * R4: the ink band. R9: it opens with the statement and the scene "the agent is not the runtime"
 * (AgentScene: the particle dissolve and the agent hand-over), then what connecting takes (the
 * one-command setup, the guide) beside the tool catalogue as two open groups on hairlines.
 */
export function Agents({ tools }: { tools: ToolLite[] }) {
  const reads = tools.filter((t) => t.annotations.readOnlyHint);
  const acts = tools.filter((t) => !t.annotations.readOnlyHint);
  return (
    <section aria-labelledby="agents-title" className="section band">
      <Reveal className="wrap">
        <div className="grid gap-6 lg:grid-cols-12 lg:gap-10">
          <h2 id="agents-title" className="t-section min-w-0 [overflow-wrap:break-word] lg:col-span-6" data-reveal>
            The agent is not the runtime
          </h2>
          <p className="t-lead min-w-0 lg:col-span-5 lg:col-start-8" data-reveal>
            Your coding agent drives the runtime over MCP. Agents come and go; the emulator and your app keep running.
          </p>
        </div>
        <div className="mt-12" data-reveal>
          <AgentScene />
        </div>
        <div className="mt-16 grid gap-12 border-t border-band-line pt-14 lg:grid-cols-12 lg:gap-10">
        <div className="min-w-0 lg:col-span-5">
          <h3 className="t-sub text-band-ink [overflow-wrap:break-word]" data-reveal>
            Your coding agent, connected over MCP
          </h3>
          <p className="t-lead mt-5" data-reveal>
            One command writes the agent&apos;s MCP entry and its instructions, for Claude Code, Codex, Cursor, Antigravity, Claude Desktop or VS
            Code. It shows you the exact change first, keeps your other servers, and undoes itself with --remove.
          </p>
          <div className="mt-8 grid gap-2" data-reveal>
            {["tumr agent setup claude-code", "tumr agent setup cursor", "tumr agent doctor"].map((c) => (
              <pre key={c} className="overflow-x-auto rounded-md border border-band-line bg-band-sunken px-4 py-3 font-mono text-sm text-band-ink">
                {c}
              </pre>
            ))}
          </div>
          <Link href="/docs/guides/agents/" className="btn btn-primary mt-8" data-reveal>
            Read the agent guide
            <ArrowRight aria-hidden className="btn-icon size-4" />
          </Link>
        </div>
        <div className="grid min-w-0 content-start gap-10 lg:col-span-6 lg:col-start-7" data-reveal>
          <ToolGroup title="Reads only" count={reads.length} note="Status, problems, logs, the screen and the UI tree." tools={reads} />
          <ToolGroup title="Acts on the device or app" count={acts.length} note="Taps, typing, rotation, reload, run and stop. A device switch asks you first." tools={acts} />
          <p className="border-t border-band-line pt-5 text-sm text-band-ink-2">
            Every agent gets its own token, limited permissions and an audit log. Output is sanitised before the agent sees it.
          </p>
        </div>
        </div>
      </Reveal>
    </section>
  );
}

function ToolGroup({ title, count, note, tools }: { title: string; count: number; note: string; tools: ToolLite[] }) {
  return (
    <div className="border-t border-band-line pt-5">
      <div className="flex flex-wrap items-baseline justify-between gap-x-6 gap-y-1">
        <h3 className="text-lg font-semibold text-band-ink">{title}</h3>
        <p className="t-num text-sm text-band-ink-2">{count} tools</p>
      </div>
      <p className="mt-1 text-sm text-band-ink-2">{note}</p>
      <ul className="mt-4 flex flex-wrap gap-2">
        {tools.map((t) => (
          <li key={t.name}>
            <Link
              href={`/docs/reference/mcp/#${t.name}`}
              className="inline-flex min-h-8 items-center rounded-sm border border-band-line bg-band-sunken px-2.5 font-mono text-[0.8125rem] text-band-ink transition-colors hover:border-band-accent hover:text-band-accent"
            >
              {t.name}
            </Link>
          </li>
        ))}
      </ul>
    </div>
  );
}
