import { Reveal } from "@/components/motion/Reveal";
import { Workflow } from "../Workflow";
import { stages } from "../workflow-data";
import { SectionHead } from "./SectionHead";

/** R4: split heading over the workflow tabs (D6: tabs only, no scroll-linking, no pin). */
export function HowItWorks() {
  return (
    <section aria-labelledby="how-title" className="section border-t border-line bg-bg-alt">
      <Reveal className="wrap">
        <SectionHead
          id="how-title"
          label="How it works"
          layout="split"
          title="From a folder to a fixed bug in eight stages"
          lead="Pick a stage to see what runs, who runs it and which parts take part. Stages one to five are yours; six to eight are the agent's loop."
        />
        <div className="mt-14" data-reveal>
          <Workflow />
          <noscript>
            <ol className="mt-8 grid gap-4">
              {stages.map((s, i) => (
                <li key={s.id} className="border-t border-line pt-5">
                  <p className="t-label">Stage {i + 1}</p>
                  <h3 className="t-sub mt-1">{s.title}</h3>
                  <p className="t-body mt-2">{s.body}</p>
                  <pre className="mt-3 font-mono text-sm">{s.command}</pre>
                </li>
              ))}
            </ol>
          </noscript>
        </div>
      </Reveal>
    </section>
  );
}
