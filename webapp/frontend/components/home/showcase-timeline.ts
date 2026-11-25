import { gsap } from "@/design-system/motion";

/**
 * R5b's device showcase cycle (standard, compact, tablet, fold closing back into the phone).
 * R8 replaced it in the hero with the 3D device lab; kept here so the showcase (HeroShowcase)
 * can be animated wherever it is used again: call showcaseTimeline(root) on an element that
 * contains HeroShowcase, then play the returned timeline (it repeats; pause it off screen).
 */
/**
 * The showcase cycle. Every state is its own element on one centre; a change of state is a FLIP
 * morph: the outgoing device scales towards the incoming one's size while fading, the incoming
 * one grows from the outgoing size. Sizes are the stage units from globals.css (.dv-*).
 */
export function showcaseTimeline(el: HTMLElement) {
  const q = gsap.utils.selector(el);
  const [phone] = q('[data-dv="standard"]');
  const [compact] = q('[data-dv="compact"]');
  const [tablet] = q('[data-dv="tablet"]');
  const [fold] = q('[data-dv="fold"]');
  const [right] = q("[data-fold-right]");
  const [left] = q("[data-fold-left]");
  const [hinge] = q("[data-fold-hinge]");
  const [shadow] = q("[data-fold-shadow]");
  const [label] = q("[data-sc-profile]");
  if (!phone || !compact || !tablet || !fold || !right || !left || !hinge || !shadow || !label) return null;

  // stage units: width x height of each device's body (keep in step with .dv-* in globals.css)
  const size = new Map<Element, [number, number]>([
    [phone, [236, 486]],
    [compact, [206, 404]],
    [tablet, [552, 386]],
    [fold, [400, 436]],
  ]);
  const CLOSED: [number, number] = [200, 436]; // the folded device: one panel
  const HOLD = 3;
  const MORPH = 0.8;
  const FOLD = 0.9;

  const setLabel = (name: string) => () => {
    label.textContent = name;
  };

  const tl = gsap.timeline({ repeat: -1, paused: true, defaults: { ease: "power3.inOut" } });
  gsap.set([compact, tablet, fold], { autoAlpha: 0 });

  const morph = (from: Element, to: Element, at: number, fromSize = size.get(from)!) => {
    const toSize = size.get(to)!;
    const grow = { scaleX: toSize[0] / fromSize[0], scaleY: toSize[1] / fromSize[1] };
    const shrink = { scaleX: fromSize[0] / toSize[0], scaleY: fromSize[1] / toSize[1] };
    tl.to(from, { ...grow, duration: MORPH }, at)
      .to(from, { autoAlpha: 0, duration: MORPH * 0.5, ease: "power1.in" }, at + MORPH * 0.15)
      .fromTo(to, { ...shrink }, { scaleX: 1, scaleY: 1, duration: MORPH, immediateRender: false }, at)
      .fromTo(to, { autoAlpha: 0 }, { autoAlpha: 1, duration: MORPH * 0.55, ease: "power1.out", immediateRender: false }, at + MORPH * 0.3)
      .set(from, { scaleX: 1, scaleY: 1 }, at + MORPH);
  };

  let t = HOLD;
  morph(phone, compact, t);
  tl.call(setLabel("compact"), [], t + MORPH * 0.5);
  t += MORPH + HOLD;
  morph(compact, tablet, t);
  tl.call(setLabel("tablet"), [], t + MORPH * 0.5);
  t += MORPH + HOLD;
  morph(tablet, fold, t);
  tl.call(setLabel("fold"), [], t + MORPH * 0.5);
  t += MORPH + HOLD;

  // the foldable closes: the right panel turns 180° on the hinge and the device slides so the
  // closed half sits on the centre; its back (the cover display) faces the viewer at the end
  tl.fromTo(right, { rotationY: 0 }, { rotationY: -180, duration: FOLD, ease: "power2.inOut", immediateRender: false }, t)
    .fromTo(fold, { xPercent: 0 }, { xPercent: 25, duration: FOLD, ease: "power2.inOut", immediateRender: false }, t)
    .to(hinge, { autoAlpha: 0, duration: 0.2 }, t)
    .to(shadow, { autoAlpha: 0, duration: FOLD * 0.6 }, t) // the cover display has its own shadow
    .set(left, { autoAlpha: 0 }, t + FOLD) // the closed device is the cover display alone
    .set(fold, { transformOrigin: "25% 50%" }, t + FOLD);
  t += FOLD + 0.35;

  // the folded device becomes the standard phone; the foldable resets out of sight
  morph(fold, phone, t, CLOSED);
  tl.call(setLabel("standard"), [], t + MORPH * 0.5);
  tl.set(right, { rotationY: 0 }, t + MORPH)
    .set([left, hinge, shadow], { autoAlpha: 1 }, t + MORPH)
    .set(fold, { xPercent: 0, transformOrigin: "50% 50%" }, t + MORPH);
  return tl;
}
