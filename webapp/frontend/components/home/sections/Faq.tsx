import { Reveal } from "@/components/motion/Reveal";
import { SectionHead } from "./SectionHead";

const faq = [
  { q: "Does it replace Android Studio?", a: "No. It runs next to it. It uses its own adb server and its own SDK folder, so Android Studio and your existing SDKs keep working as before." },
  {
    q: "Does it work in VS Code?",
    // R4: the store sentence was out of date (the extension is published since 0.1.5).
    a: "Yes. The Toolchain Universal Mobile Runtime extension runs your app on the emulator from the Command Palette, shows the phone in a Mobile Preview panel beside your code, puts build errors and crashes in the Problems panel and connects Copilot's agent mode. It is on the VS Code Marketplace and Open VSX, and also installs from a .vsix file. It needs the runtime installed.",
  },
  { q: "Can it run iOS apps?", a: "No. The iOS Simulator needs macOS and Xcode. This runtime is for Windows 11 x64 and Android today." },
  { q: "Which coding agents does it work with?", a: "Claude Code, Codex, Cursor, Antigravity, Claude Desktop and VS Code each connect with one command, tumr agent setup, and tumr agent doctor checks the connection. Any other MCP client can start tumr mcp by hand. The claude.ai website and mobile app cannot reach a program on your PC." },
  { q: "Does my code leave my machine?", a: "No. The runtime has no cloud service. It downloads tools from their official publishers and nothing else, unless you run tumr update." },
  { q: "What does it need from my PC?", a: "Windows 11 x64, virtualisation turned on in the firmware, and room for the toolchain and one phone image. The requirements page lists each level." },
  { q: "How do I remove it?", a: "Uninstall it from Windows Settings. The uninstall guide lists every folder it created and how to remove the downloaded tools too." },
];

/**
 * R4: questions set as type, not boxes: large questions on hairlines, answers at reading measure.
 * R10: answers open and close with a short height transition where the browser supports
 * ::details-content (CSS only, no script; instant elsewhere and under reduced motion), and the
 * question shows a clear focus ring for keyboard users.
 */
export function Faq() {
  return (
    <section aria-labelledby="faq-title" className="section border-t border-line">
      <Reveal className="wrap grid gap-10 lg:grid-cols-12">
        <div className="min-w-0 lg:col-span-4">
          <SectionHead id="faq-title" label="Questions" title="Before you install" />
        </div>
        <div className="divide-y divide-line border-y border-line-strong lg:col-span-8">
          {faq.map((f) => (
            <details key={f.q} className="faq-item group" data-reveal>
              <summary className="faq-q flex min-h-16 cursor-pointer list-none items-center justify-between gap-6 py-5 text-[1.1875rem] font-medium tracking-[-0.01em] text-ink transition-colors hover:text-signal-ink [&::-webkit-details-marker]:hidden">
                {f.q}
                <span aria-hidden className="grid size-8 shrink-0 place-items-center rounded-full border border-line-strong text-ink-2 transition-transform group-open:rotate-45">
                  +
                </span>
              </summary>
              <p className="t-body max-w-[42rem] pb-6 text-ink-2">{f.a}</p>
            </details>
          ))}
        </div>
      </Reveal>
    </section>
  );
}
