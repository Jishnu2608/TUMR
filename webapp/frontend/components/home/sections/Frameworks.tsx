import Link from "next/link";
import { Reveal } from "@/components/motion/Reveal";
import { FrameworkTabs } from "../FrameworkTabs";
import { SectionHead } from "./SectionHead";

/**
 * R4: split heading. R7: the comparison became one tab per framework with a panel of plain,
 * neutral facts (language, detection, installs, build and run, after you save, problems, limits),
 * all from the framework guides; readable without script (FrameworkTabs).
 */
export function Frameworks() {
  return (
    <section aria-labelledby="frameworks-title" className="section border-t border-line">
      <Reveal className="wrap">
        <SectionHead
          id="frameworks-title"
          label="Frameworks"
          layout="split"
          title="Kotlin, Java, Flutter, React Native and Expo"
          lead="One adapter per toolchain: Gradle for Kotlin and Java, Flutter, and React Native for React Native and Expo. Each uses the framework's own tools, so a project that builds in Android Studio or the Flutter CLI builds here."
        />
        <div className="mt-14" data-reveal>
          <FrameworkTabs />
        </div>
        <p className="t-desc mt-6" data-reveal>
          Framework guides:{" "}
          <Link className="link" href="/docs/guides/kotlin-java/">Kotlin and Java</Link>,{" "}
          <Link className="link" href="/docs/guides/flutter/">Flutter</Link>,{" "}
          <Link className="link" href="/docs/guides/react-native-expo/">React Native and Expo</Link>.
        </p>
      </Reveal>
    </section>
  );
}
