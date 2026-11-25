import Link from "next/link";
import { Puzzle } from "lucide-react";
import { Reveal } from "@/components/motion/Reveal";
import { site } from "@/lib/site";

/* The VS Code extension on the home page. */

const features = [
  { title: "Run on Emulator", body: "From the Command Palette or the status bar: checks the toolchain, boots the emulator, builds, installs and launches the open folder." },
  { title: "Mobile Preview panel", body: "The live phone beside your code: click and type, rotate, screenshot, device sizes, and the in-display fingerprint sensor." },
  { title: "Saved files apply themselves", body: "Flutter hot reload, React Native Fast Refresh, an incremental Gradle rebuild for Kotlin and Java." },
  { title: "Problems panel", body: "Build errors, runtime errors and crashes of the running app, with file and line, next to your compiler's own." },
  { title: "Copilot agent mode", body: "Connect Copilot gives VS Code's agent the runtime's tools: status, problems, screenshot, tap, reload, fingerprint." },
];

/** R10: the second store (Cursor, Windsurf and VSCodium install from Open VSX). */
function OpenVsxButton() {
  const o = site.vscode.openVsx;
  if (!o.published) return null;
  return (
    <a href={o.url} className="btn btn-secondary btn-lg" rel="noopener">
      Open VSX
    </a>
  );
}

/** The store button: a link once the extension is published, a labelled placeholder until then. */
export function MarketplaceButton({ className = "" }: { className?: string }) {
  const m = site.vscode.marketplace;
  if (m.published) {
    return (
      <a href={m.url} className={`btn btn-wrap btn-primary btn-lg ${className}`} rel="noopener">
        <Puzzle aria-hidden className="size-4" />
        Get it on the VS Code Marketplace
      </a>
    );
  }
  return (
    <span className={`btn btn-secondary btn-lg cursor-not-allowed opacity-70 ${className}`} aria-disabled="true" title="Not published yet">
      <Puzzle aria-hidden className="size-4" />
      VS Code Marketplace, coming soon
    </span>
  );
}

export function VsCode() {
  return (
    <section aria-labelledby="vscode-title" className="section border-t border-line" id="vscode">
      {/* R4: picture first on wide screens (the Agents band before it leads with text), the
          features as a list on hairlines instead of dotted bullets */}
      <Reveal className="wrap grid items-center gap-12 lg:grid-cols-12 lg:gap-10">
        <div className="min-w-0 lg:order-2 lg:col-span-5 lg:col-start-8">
          <header data-reveal>
            <h2 id="vscode-title" className="t-section">
              Your Android app inside VS Code
            </h2>
            <p className="t-lead mt-5">
              One extension for Kotlin, Java, Flutter and React Native: run the app on the emulator, see the phone in a panel beside your
              code, and get its errors where your compiler&apos;s already are.
            </p>
          </header>
          <ul className="mt-8 border-t border-line" data-reveal>
            {features.map((f) => (
              <li key={f.title} className="border-b border-line py-3.5">
                <span className="font-semibold text-ink">{f.title}.</span> <span className="text-ink-2">{f.body}</span>
              </li>
            ))}
          </ul>
          <div className="mt-9 flex flex-wrap items-center gap-3" data-reveal>
            <MarketplaceButton />
            <OpenVsxButton />
          </div>
          <p className="t-desc mt-4" data-reveal>
            Or install the <code>{site.vscode.vsix}</code> file from the{" "}
            <Link className="link" href="/download/#vscode">
              download page
            </Link>
            .{" "}
            <Link className="link" href="/docs/guides/vscode/">
              Read the VS Code guide
            </Link>
            .
          </p>
          <p className="t-desc mt-2" data-reveal>
            Needs the runtime installed (<Link className="link" href="/download/">download page</Link>). Try it with the{" "}
            <a className="link" href={`${site.repository}/tree/main/sample-app`} rel="noopener">
              sample calculators
            </a>{" "}
            in Kotlin, Java, Dart and React Native.
          </p>
        </div>
        <div className="min-w-0 lg:order-1 lg:col-span-7" data-reveal>
          <EditorShot />
        </div>
      </Reveal>
    </section>
  );
}

/**
 * Real captures of VS Code with the extension (WEB-015). R10: a list, so later captures (Flutter,
 * React Native, the Problems panel up close) are added here; the first is the one shown (a picker
 * between them is for when there is more than one).
 */
const captures = [
  {
    src: "/images/vscode-mobile-preview.png",
    width: 1400,
    height: 1006,
    label: "Kotlin",
    alt: "VS Code with Calculator.kt open in the editor, the running calculator app in the Mobile Preview panel beside it, and the runtime's Problems tab below the phone",
  },
];

function EditorShot() {
  const c = captures[0];
  return (
    <figure className="editor-shot">
      {/* eslint-disable-next-line @next/next/no-img-element -- static export, pre-sized PNG */}
      <img src={c.src} width={c.width} height={c.height} loading="lazy" decoding="async" className="block h-auto w-full" alt={c.alt} />
      <figcaption className="editor-shot-cap">Real capture: the sample Kotlin app running in the Mobile Preview panel</figcaption>
    </figure>
  );
}
