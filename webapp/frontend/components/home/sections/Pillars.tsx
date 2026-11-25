import { FileSearch, MonitorSmartphone, PlugZap, Wrench } from "lucide-react";
import { Reveal } from "@/components/motion/Reveal";
import { MotifPlayer } from "@/components/motion/MotifPlayer";
import { DockMotif, ProblemsMotif, ProfilesMotif, ToolchainMotif } from "./PillarMotifs";
import { SectionHead } from "./SectionHead";

const pillars = [
  {
    icon: Wrench,
    title: "Toolchain from the project",
    body: "It reads the Gradle wrapper, Flutter and Node constraints, then installs only what is missing: JDK, Android SDK, emulator, Flutter, Node. Sizes and licences are shown before anything downloads.",
    fact: "No administrator rights for the installs",
    Motif: ToolchainMotif,
  },
  {
    icon: MonitorSmartphone,
    title: "An emulator that stays out of the way",
    body: "Phones boot headless from a snapshot on the runtime's own adb server, so Android Studio and your existing SDKs keep working. Six phone and tablet profiles, from compact to tablet.",
    fact: "Global PATH, JAVA_HOME and ANDROID_HOME untouched",
    Motif: ProfilesMotif,
  },
  {
    icon: PlugZap,
    title: "Your app beside your agent",
    body: "The running app streams into a window you dock next to Claude Code or Codex. You tap and type in it; the agent sees the same screen through screenshots and the UI tree.",
    fact: "Flutter hot reload 0.7 to 1.8 s, measured",
    Motif: DockMotif,
  },
  {
    icon: FileSearch,
    title: "Problems an agent can act on",
    body: "Build errors, crashes and runtime exceptions are classified, deduplicated and located at file and line, and marked as your code or the environment.",
    fact: "Tested on 76 seeded failures across six categories",
    Motif: ProblemsMotif,
  },
];

/**
 * R4: the four capabilities as an editorial ledger (numbered rows on hairlines), not a card grid.
 * R6: each row carries its own motif under the title, each moving differently (MotifPlayer).
 */
export function Pillars() {
  return (
    <section aria-labelledby="pillars-title" className="section border-t border-line">
      <Reveal className="wrap">
        <SectionHead
          id="pillars-title"
          label="What it does"
          layout="split"
          title="Four jobs, one local runtime"
          lead="Everything an agent needs to see a mobile app run, without you learning the Android toolchain first."
        />
        <MotifPlayer>
        <ol className="mt-14 border-t border-line-strong">
          {pillars.map((p, i) => (
            <li key={p.title} data-reveal className="grid gap-4 border-b border-line py-8 sm:py-10 lg:grid-cols-12 lg:gap-10">
              <div className="flex items-center gap-4 lg:col-span-1 lg:flex-col lg:items-start">
                <span className="t-num text-sm font-medium text-ink-2">{String(i + 1).padStart(2, "0")}</span>
                <p.icon aria-hidden className="size-6 text-signal-ink" strokeWidth={1.75} />
              </div>
              <div className="min-w-0 lg:col-span-5">
                <h3 className="t-sub">{p.title}</h3>
                <div className="mt-6 max-w-[21rem]">
                  <p.Motif />
                </div>
              </div>
              <div className="min-w-0 lg:col-span-6 lg:col-start-7">
                <p className="t-body text-ink-2">{p.body}</p>
                <p className="mt-4 text-[0.9375rem] font-medium text-ink">{p.fact}</p>
              </div>
            </li>
          ))}
        </ol>
        </MotifPlayer>
      </Reveal>
    </section>
  );
}
