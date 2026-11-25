import Link from "next/link";
import { ArrowUpRight } from "lucide-react";
import { Reveal } from "@/components/motion/Reveal";
import { site } from "@/lib/site";
import { SectionHead } from "./SectionHead";

const openLinks = [
  { title: "Source code", body: "Every part of the runtime, from the Core to the desktop app and the VS Code extension.", href: site.repository, external: true },
  { title: "Contributing", body: "How to build it, run the tests and propose a change.", href: "/open-source/#contributing" },
  { title: "Third-party licences", body: "Every component the runtime ships with, and its licence.", href: "/licences/" },
];

const rowClass =
  "group grid grid-cols-[minmax(0,1fr)_auto] items-start gap-6 border-b border-line py-5 transition-colors hover:text-signal-ink focus-visible:outline-offset-2";

function Row({ title, body }: { title: string; body: string }) {
  return (
    <>
      <span className="min-w-0">
        <span className="text-lg font-semibold text-ink group-hover:text-signal-ink">{title}</span>
        <span className="mt-1 block text-[0.95rem] text-ink-2">{body}</span>
      </span>
      <ArrowUpRight aria-hidden className="mt-1.5 size-5 shrink-0 text-ink-2 transition-transform group-hover:-translate-y-0.5 group-hover:translate-x-0.5 group-hover:text-signal-ink" />
    </>
  );
}

/** R4: an editorial block, the statement on the left and the three destinations as an index on hairlines. */
export function OpenSource() {
  return (
    <section aria-labelledby="oss-title" className="section border-t border-line">
      <Reveal className="wrap grid gap-10 lg:grid-cols-12">
        <div className="min-w-0 lg:col-span-5">
          <SectionHead
            id="oss-title"
            label="Open source"
            title="Built in the open"
            lead="The source code is on GitHub: read how the runtime works, follow its development and report what you find."
          />
        </div>
        <ul className="grid content-start border-t border-line-strong lg:col-span-6 lg:col-start-7" data-reveal>
          {openLinks.map((l) => (
            <li key={l.title}>
              {l.external ? (
                <a href={l.href} rel="noreferrer" className={rowClass}>
                  <Row title={l.title} body={l.body} />
                </a>
              ) : (
                <Link href={l.href} className={rowClass}>
                  <Row title={l.title} body={l.body} />
                </Link>
              )}
            </li>
          ))}
        </ul>
      </Reveal>
    </section>
  );
}
