"use client";

/**
 * R2 motion hooks. Later phases (hero, Device Lab, particles) share these instead of each
 * writing its own media query, observer or theme listener.
 */
import { useEffect, useRef, useState, useSyncExternalStore, type RefObject } from "react";
import { gsap, media, onThemeChange, readTokens, registerGsap, useGSAP, type MediaKey } from "@/design-system/motion";

type MediaFlags = Record<MediaKey, boolean>;

/**
 * `gsap.matchMedia()` with the site's breakpoints and motion preference. `setup` runs once per
 * matching combination and everything it creates is reverted when the match changes or the
 * component unmounts, e.g.
 *
 *   useMedia((m) => { if (m.reduced) return; gsap.from(".x", { y: 20 }); }, rootRef);
 */
export function useMedia(
  setup: (flags: MediaFlags) => void | (() => void),
  scope?: RefObject<Element | null>,
  dependencies: unknown[] = [],
) {
  useGSAP(
    () => {
      registerGsap();
      const mm = gsap.matchMedia(scope?.current ?? undefined);
      mm.add({ ...media }, (ctx) => setup(ctx.conditions as MediaFlags));
      return () => mm.revert();
    },
    { scope, dependencies },
  );
}

/**
 * True while the element is on screen (IntersectionObserver). With `once`, it stays true after
 * the first time. Starts false on the server and before the first observation.
 */
export function useInView<T extends Element>(
  options: { rootMargin?: string; threshold?: number; once?: boolean } = {},
): [RefObject<T | null>, boolean] {
  const ref = useRef<T>(null);
  const [inView, setInView] = useState(false);
  const { rootMargin = "0px", threshold = 0, once = false } = options;
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (!("IntersectionObserver" in window)) return; // no observer: reported as in view below
    const io = new IntersectionObserver(
      ([e]) => {
        setInView(e.isIntersecting);
        if (once && e.isIntersecting) io.disconnect();
      },
      { rootMargin, threshold },
    );
    io.observe(el);
    return () => io.disconnect();
  }, [rootMargin, threshold, once]);
  const unsupported = typeof window !== "undefined" && !("IntersectionObserver" in window);
  return [ref, inView || unsupported];
}

function subscribeVisibility(cb: () => void) {
  document.addEventListener("visibilitychange", cb);
  return () => document.removeEventListener("visibilitychange", cb);
}

/** False while the tab is hidden, so scenes can stop their render loop. True on the server. */
export function usePageVisible() {
  return useSyncExternalStore(
    subscribeVisibility,
    () => document.visibilityState === "visible",
    () => true,
  );
}

/** Design tokens for canvas / WebGL, re-read when the theme changes. Empty strings on the server. */
export function useTokens<T extends string>(names: readonly T[]): Record<T, string> {
  const key = names.join("|");
  const [tokens, setTokens] = useState<Record<T, string>>(() => Object.fromEntries(names.map((n) => [n, ""])) as Record<T, string>);
  useEffect(() => {
    const list = key.split("|") as T[];
    const read = () => setTokens(readTokens(list));
    read();
    return onThemeChange(read);
  }, [key]);
  return tokens;
}
