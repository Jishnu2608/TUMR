"use client";

import { useEffect, useRef } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { ArrowRight, Menu, X } from "lucide-react";
import { nav, site } from "@/lib/site";
import { Wordmark } from "./Wordmark";
import { ThemeToggle } from "@/components/theme/ThemeToggle";
import { SearchButton } from "./Search";
import { cn } from "@/lib/cn";

function isActive(path: string, href: string) {
  if (href === "/docs/getting-started/") return path.startsWith("/docs/") && !path.startsWith("/docs/requirements");
  return path.startsWith(href);
}

/**
 * R3 (Studio / Signal): a compact header on the canvas colour. Active page = Signal Blue rule on
 * the header's bottom edge; the download action is the one Signal Blue button. The phone menu is
 * a <details> element, so it works without script; with script it also closes on Escape, on a
 * click outside and after a navigation.
 */
export function SiteHeader({ hasRelease }: { hasRelease: boolean }) {
  const path = usePathname() ?? "/";
  const cta = hasRelease ? "Download for Windows" : "See downloads";
  const menu = useRef<HTMLDetailsElement>(null);

  useEffect(() => {
    const close = (focusSummary: boolean) => {
      const d = menu.current;
      if (!d?.open) return;
      d.open = false;
      if (focusSummary) d.querySelector("summary")?.focus();
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close(true);
    const onPointer = (e: PointerEvent) => {
      if (menu.current && !menu.current.contains(e.target as Node)) close(false);
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("pointerdown", onPointer);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("pointerdown", onPointer);
    };
  }, []);

  // a route change closes the menu
  useEffect(() => {
    if (menu.current) menu.current.open = false;
  }, [path]);

  return (
    <header className="sticky top-0 z-50 border-b border-line bg-header/90 backdrop-blur-md supports-[backdrop-filter]:bg-header/80">
      <div className="wrap flex h-[var(--header-h)] items-center gap-3 sm:gap-6">
        {/* on the home page itself the logo does not prefetch: that pulled the page's own 76 KB RSC
            payload and parsed it on the main thread during load (WEB-017) */}
        <Link
          href="/"
          prefetch={path === "/" ? false : undefined}
          className="shrink-0 rounded-sm"
          aria-label={`${site.shortName}, ${site.name}, home`}
        >
          <Wordmark />
        </Link>

        <nav aria-label="Main" className="hidden h-full min-[1200px]:block">
          <ul className="flex h-full items-center gap-0.5">
            {nav.map((n) => {
              const active = isActive(path, n.href);
              return (
                <li key={n.href} className="h-full">
                  <Link
                    href={n.href}
                    aria-current={active ? "page" : undefined}
                    className={cn(
                      "relative inline-flex h-full items-center whitespace-nowrap px-2.5 text-[0.9375rem] text-ink-2 transition-colors hover:text-ink",
                      "after:absolute after:inset-x-3 after:-bottom-px after:h-[2px] after:origin-left after:rounded-full after:transition-transform after:duration-300",
                      active
                        ? "font-medium text-ink after:scale-x-100 after:bg-signal"
                        : "after:scale-x-0 after:bg-line-strong hover:after:scale-x-100",
                    )}
                  >
                    {n.label}
                  </Link>
                </li>
              );
            })}
          </ul>
        </nav>

        <div className="ml-auto flex items-center gap-0.5 sm:gap-1">
          <SearchButton />
          <ThemeToggle />
          <Link href="/download/" className="btn btn-primary btn-sm ml-2 hidden md:inline-flex">
            {cta}
            <ArrowRight aria-hidden className="btn-icon size-4" />
          </Link>

          {/* phone menu: <details> works without script */}
          <details ref={menu} className="group min-[1200px]:hidden">
            <summary
              className="flex size-11 cursor-pointer list-none items-center justify-center rounded-sm text-ink hover:bg-sunken [&::-webkit-details-marker]:hidden"
              aria-label="Menu"
            >
              <Menu aria-hidden className="size-5 group-open:hidden" />
              <X aria-hidden className="hidden size-5 group-open:block" />
            </summary>
            <div className="absolute inset-x-0 top-full z-40 max-h-[calc(100dvh-var(--header-h))] overflow-y-auto border-b border-line bg-bg shadow-2">
              <nav aria-label="Menu" className="wrap py-3">
                <ul className="divide-y divide-line">
                  {nav.map((n) => {
                    const active = isActive(path, n.href);
                    return (
                      <li key={n.href}>
                        <Link
                          href={n.href}
                          aria-current={active ? "page" : undefined}
                          className={cn(
                            "flex min-h-12 items-center justify-between border-l-2 border-transparent pl-3 text-lg font-medium text-ink",
                            active && "border-signal text-signal-ink",
                          )}
                          onClick={() => menu.current?.removeAttribute("open")}
                        >
                          {n.label}
                          <ArrowRight aria-hidden className="size-4 text-ink-2" />
                        </Link>
                      </li>
                    );
                  })}
                </ul>
                <Link href="/download/" className="btn btn-primary btn-lg mt-4 w-full">
                  {cta}
                  <ArrowRight aria-hidden className="btn-icon size-4" />
                </Link>
              </nav>
            </div>
          </details>
        </div>
      </div>
    </header>
  );
}
