import { Reveal } from "@/components/motion/Reveal";
import { Architecture } from "../Architecture";
import { SectionHead } from "./SectionHead";

/** R4: split heading over the Core diagram. */
export function ArchitectureSection() {
  return (
    <section aria-labelledby="arch-title" className="section border-t border-line">
      <div className="wrap">
        <Reveal>
          <SectionHead
            id="arch-title"
            label="Architecture"
            layout="split"
            title="A Runtime Core that outlives every window"
            lead="The command line, the desktop app and the agent are clients of one Core per Windows user. Close the app or restart the agent, and the emulator and the app keep running."
          />
        </Reveal>
        <div className="mt-14">
          <Architecture />
        </div>
      </div>
    </section>
  );
}
