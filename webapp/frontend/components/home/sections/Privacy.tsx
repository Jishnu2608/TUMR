import Link from "next/link";
import { ArrowDownToLine, Ban, FileSearch, Lock, ShieldCheck, Wrench } from "lucide-react";
import { Reveal } from "@/components/motion/Reveal";
import { SectionHead } from "./SectionHead";

const privacy = [
  { icon: Lock, title: "Local only", body: "No account, no cloud service, no telemetry. It downloads only what you approved, from the official publishers." },
  { icon: ShieldCheck, title: "Verified downloads", body: "HTTPS from an allowlist of official hosts. Checksums are checked before anything is unpacked or run." },
  { icon: Wrench, title: "Your system as it was", body: "Global PATH, JAVA_HOME, ANDROID_HOME and existing SDKs are never changed. Two named actions may ask for administrator rights." },
  { icon: FileSearch, title: "Project trust", body: "Building runs the project's own scripts. Nothing builds until you trust the folder, and agents cannot trust folders." },
];

/**
 * R4: privacy as a boundary diagram. The four guarantees sit inside one dashed line, the edge of
 * the PC, drawn from the section's own facts (local only, nothing leaves); no new claims.
 * R10: the boundary says what crosses it: one way in (downloads you approved, from official
 * hosts, checked), and nothing out (no account, no cloud service, no telemetry).
 */
export function Privacy() {
  return (
    <section aria-labelledby="privacy-title" className="section border-t border-line bg-bg-alt">
      <Reveal className="wrap">
        <SectionHead id="privacy-title" label="Privacy and security" title="Your code and your machine stay yours" />
        <div className="relative mt-14 rounded-lg border border-dashed border-line-strong p-5 sm:p-8 lg:p-10" data-reveal>
          <p className="absolute -top-3 left-5 bg-bg-alt px-2 text-sm font-medium text-ink sm:left-8">Your PC</p>
          <div className="privacy-edge" aria-label="What crosses the edge of your PC">
            <p className="privacy-cross">
              <ArrowDownToLine aria-hidden className="size-4" />
              In: only the downloads you approved, from official hosts, checksums checked
            </p>
            <p className="privacy-cross">
              <Ban aria-hidden className="size-4" />
              Out: nothing. No account, no cloud service, no telemetry
            </p>
          </div>
          <ul className="grid gap-px overflow-hidden rounded-md bg-line sm:grid-cols-2">
            {privacy.map((p) => (
              <li key={p.title} className="bg-bg-alt p-5 sm:p-7">
                <p.icon aria-hidden className="size-5 text-signal-ink" strokeWidth={1.75} />
                <h3 className="mt-4 text-lg font-semibold text-ink">{p.title}</h3>
                <p className="t-desc mt-2 max-w-[34rem]">{p.body}</p>
              </li>
            ))}
          </ul>
        </div>
        <div className="mt-10 flex flex-wrap gap-3" data-reveal>
          <Link href="/privacy/" className="btn btn-secondary">Read the privacy notes</Link>
          <Link href="/security/" className="btn btn-ghost">How the runtime protects you</Link>
        </div>
      </Reveal>
    </section>
  );
}
