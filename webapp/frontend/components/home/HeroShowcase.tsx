import { CompactScreen, CoverScreen, FoldDetailScreen, FoldListScreen, PhoneScreen, TabletScreen } from "./SampleTravelApp";

/**
 * R5b: the device showcase. Four of the runtime's device profiles (standard, compact, tablet,
 * fold) stacked on one centre; the Hero component morphs between them with transforms and
 * opacity only (GSAP), and folds the foldable shut with a CSS 3D hinge. Without script, or with
 * reduced motion, the standard phone stands alone (the static fallback). Device sizes are in
 * stage units (600 x 560).
 *
 * R8: no longer shown in the hero (the 3D device lab took its place); kept, with its cycle in
 * `showcase-timeline.ts`, for use elsewhere. Its stage (ShowcaseStage) is the lab's static picture.
 */
export function HeroShowcase() {
  return (
    <figure
      className="showcase"
      aria-label="A sample travel app shown on the runtime's standard, compact, tablet and foldable device profiles"
    >
      <ShowcaseStage />

      <figcaption className="sc-caption">
        <span className="sc-profile">
          <span className="sc-led" aria-hidden />
          Device profile <code data-sc-profile>standard</code>
        </span>
        <span className="sc-credit">
          Sample app for illustration. Photos:{" "}
          <a title="CC BY 4.0" href="https://commons.wikimedia.org/wiki/File:Tram_28_at_Largo_de_Santa_Luzia_in_Lisbon,_20250606_1958_0419.jpg" rel="noreferrer">
            Jakub Hałun
          </a>
          ,{" "}
          <a title="CC BY 3.0" href="https://commons.wikimedia.org/wiki/File:Herbert_Lake_beim_Icefields_Parkway.jpg" rel="noreferrer">
            Florian Fuchs
          </a>
          ,{" "}
          <a
            title="CC BY-SA 4.0"
            href="https://commons.wikimedia.org/wiki/File:Wooden_gate_with_open_double_door_leading_to_the_garden_of_Isshinin_Buddhist_temple_in_the_compounds_of_Chion-in_Kyoto_Japan.jpg"
            rel="noreferrer"
          >
            Basile Morin
          </a>
          ,{" "}
          <a title="Public domain" href="https://commons.wikimedia.org/wiki/File:Utah_Dunes_Landscape_-_West_Desert_District.jpg" rel="noreferrer">
            BLM Utah
          </a>{" "}
          (Wikimedia Commons)
        </span>
      </figcaption>
    </figure>
  );
}

/**
 * The showcase stage alone: the four devices on one centre. In the hero since R8 it is the device
 * lab's static picture (only the standard phone shows), replaced by the 3D lab once that loads.
 */
export function ShowcaseStage() {
  return (
    <div className="sc-stage" aria-hidden data-sc-stage>

        <div className="sc-scroll" data-sc-scroll>
          <div className="dv dv-phone" data-dv="standard">
            <div className="dv-screen">
              <PhoneScreen />
            </div>
          </div>

          <div className="dv dv-compact" data-dv="compact">
            <div className="dv-screen">
              <CompactScreen />
            </div>
          </div>

          <div className="dv dv-tablet" data-dv="tablet">
            <div className="dv-screen">
              <TabletScreen />
            </div>
          </div>

          {/* foldable: the right panel turns on the hinge (its left edge); its back is the cover display */}
          <div className="dv-fold" data-dv="fold">
            <span className="dv-fold-shadow" data-fold-shadow />
            <div className="dv-panel dv-panel-left" data-fold-left>
              <div className="dv-screen">
                <FoldListScreen />
              </div>
            </div>
            <div className="dv-panel dv-panel-right" data-fold-right>
              <div className="dv-face dv-face-front">
                <div className="dv-screen">
                  <FoldDetailScreen />
                </div>
              </div>
              <div className="dv-face dv-face-back">
                <div className="dv-screen">
                  <CoverScreen />
                </div>
              </div>
            </div>
            <span className="dv-hinge" data-fold-hinge />
          </div>
        </div>
      </div>
  );
}
