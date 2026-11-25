import Link from "next/link";
import { BookOpen, Download } from "lucide-react";
import { Reveal } from "@/components/motion/Reveal";

/**
 * R4: the story's end. Left-aligned: the line on the left, the two actions on the right.
 * R10: it answers the hero ("Your agent writes the code. TUMR brings it to life.") in the same
 * voice; the restrained visual is the device-profile row in type, no new picture.
 */
export function Close({ hasRelease }: { hasRelease: boolean }) {
  return (
    <section aria-labelledby="close-title" className="section band">
      <Reveal className="wrap grid gap-10 lg:grid-cols-12 lg:items-end">
        <h2 id="close-title" className="t-page min-w-0 [overflow-wrap:break-word] lg:col-span-7" data-reveal>
          Your agent writes the code. Give it a phone to test on.
        </h2>
        <div className="min-w-0 lg:col-span-5" data-reveal>
          <p className="t-lead">Windows 11 x64. Kotlin, Java, Flutter, React Native and Expo.</p>
          <p className="close-profiles" aria-label="Device profiles">
            {["compact", "standard", "large", "fold", "flip", "tablet"].map((p) => (
              <code key={p}>{p}</code>
            ))}
          </p>
          <div className="mt-8 flex flex-wrap gap-3">
            <Link href="/download/" className="btn btn-primary btn-lg">
              <Download aria-hidden className="size-4" />
              {hasRelease ? "Download for Windows" : "See downloads"}
            </Link>
            <Link href="/docs/getting-started/" className="btn btn-secondary btn-lg">
              <BookOpen aria-hidden className="size-4" />
              Read the getting-started guide
            </Link>
          </div>
        </div>
      </Reveal>
    </section>
  );
}
