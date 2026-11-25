import { Bell, Compass, Heart, MapPin, Map as MapIcon, Search, Star, User } from "lucide-react";

/**
 * R5b: a sample travel app ("Wayfarer"), rendered locally for the hero showcase. It is an
 * illustration of an app someone could build and run on the runtime's device profiles, not an
 * app the runtime ships. Each layout is designed for its screen (no stretched screenshot).
 * Sizes use the showcase unit `--u` (1/600 of the stage width).
 */

const IMG = "/images/sample-app";
const places = [
  { id: "lisbon", name: "Lisbon", country: "Portugal", rating: "4.8", img: `${IMG}/lisbon.webp`, w: 440, h: 660 },
  { id: "lake", name: "Herbert Lake", country: "Canada", rating: "4.9", img: `${IMG}/lake.webp`, w: 420, h: 280 },
  { id: "kyoto", name: "Kyoto", country: "Japan", rating: "4.9", img: `${IMG}/kyoto.webp`, w: 420, h: 315 },
  { id: "dunes", name: "West Desert", country: "Utah", rating: "4.7", img: `${IMG}/dunes.webp`, w: 420, h: 276 },
] as const;
type Place = (typeof places)[number];

function Photo({ p, className, pos }: { p: Place; className?: string; pos?: string }) {
  return (
    // eslint-disable-next-line @next/next/no-img-element -- static export, pre-sized WebP
    <img
      src={p.img}
      width={p.w}
      height={p.h}
      alt=""
      loading="lazy"
      decoding="async"
      className={`ta-img ${className ?? ""}`}
      style={pos ? { objectPosition: pos } : undefined}
    />
  );
}

function StatusBar() {
  return (
    <div className="ta-status" aria-hidden>
      <span>9:41</span>
      <span className="ta-status-icons">
        <i />
        <i />
        <i className="ta-batt" />
      </span>
    </div>
  );
}

function Chips({ n = 4 }: { n?: number }) {
  return (
    <div className="ta-chips">
      {["Cities", "Nature", "Coast", "Culture"].slice(0, n).map((c, i) => (
        <span key={c} className={i === 0 ? "ta-chip ta-chip-on" : "ta-chip"}>
          {c}
        </span>
      ))}
    </div>
  );
}

function Feature({ p, tall }: { p: Place; tall?: boolean }) {
  return (
    <div className={tall ? "ta-feature ta-feature-tall" : "ta-feature"}>
      <Photo p={p} pos="50% 60%" />
      <span className="ta-save">
        <Heart className="ta-ico" />
      </span>
      <div className="ta-feature-text">
        <span className="ta-feature-name">{p.name}</span>
        <span className="ta-feature-meta">
          <MapPin className="ta-ico-s" />
          {p.country}
          <Star className="ta-ico-s ta-star" />
          {p.rating}
        </span>
      </div>
    </div>
  );
}

function SmallCard({ p }: { p: Place }) {
  return (
    <div className="ta-small">
      <Photo p={p} />
      <span className="ta-small-name">{p.name}</span>
      <span className="ta-small-meta">{p.country}</span>
    </div>
  );
}

function BottomNav() {
  return (
    <div className="ta-nav">
      <span className="ta-nav-on">
        <Compass className="ta-ico" />
        Explore
      </span>
      <span>
        <Heart className="ta-ico" />
        Saved
      </span>
      <span>
        <MapIcon className="ta-ico" />
        Trips
      </span>
      <span>
        <User className="ta-ico" />
        Profile
      </span>
    </div>
  );
}

function Header({ compact }: { compact?: boolean }) {
  return (
    <div className="ta-head">
      <div>
        <span className="ta-hello">Good morning, Ana</span>
        <span className={compact ? "ta-title ta-title-s" : "ta-title"}>Where to next?</span>
      </div>
      <span className="ta-bell">
        <Bell className="ta-ico" />
      </span>
    </div>
  );
}

function SearchBar() {
  return (
    <div className="ta-search">
      <Search className="ta-ico" />
      Search destinations
    </div>
  );
}

/** Standard phone: header, search, chips, a tall feature card, a row of nearby escapes, tab bar. */
export function PhoneScreen() {
  return (
    <div className="ta ta-phone">
      <StatusBar />
      <Header />
      <SearchBar />
      <Chips />
      <Feature p={places[0]} />
      <div className="ta-row-head">
        <span>Nearby escapes</span>
        <span className="ta-link">See all</span>
      </div>
      <div className="ta-row">
        <SmallCard p={places[1]} />
        <SmallCard p={places[2]} />
      </div>
      <BottomNav />
    </div>
  );
}

/** Compact phone: the same app with less at once: shorter header, three chips, the feature only. */
export function CompactScreen() {
  return (
    <div className="ta ta-compact">
      <StatusBar />
      <Header compact />
      <SearchBar />
      <Chips n={3} />
      <Feature p={places[0]} tall />
      <BottomNav />
    </div>
  );
}

/** Tablet (landscape): navigation rail, a wide feature beside a column of destinations. */
export function TabletScreen() {
  return (
    <div className="ta ta-tablet">
      <StatusBar />
      <div className="ta-tablet-body">
        <div className="ta-rail">
          <span className="ta-rail-logo">W</span>
          <span className="ta-rail-on">
            <Compass className="ta-ico" />
          </span>
          <Heart className="ta-ico" />
          <MapIcon className="ta-ico" />
          <User className="ta-ico" />
        </div>
        <div className="ta-tablet-main">
          <div className="ta-tablet-top">
            <Header />
            <SearchBar />
          </div>
          <Chips />
          <div className="ta-tablet-grid">
            <Feature p={places[0]} />
            <div className="ta-tablet-list">
              {places.slice(1).map((p) => (
                <SmallCard key={p.id} p={p} />
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

/** Foldable, inner left panel: the destination list. */
export function FoldListScreen() {
  return (
    <div className="ta ta-fold-list">
      <StatusBar />
      <Header compact />
      <SearchBar />
      <div className="ta-list">
        {places.map((p, i) => (
          <div key={p.id} className={i === 0 ? "ta-list-row ta-list-on" : "ta-list-row"}>
            <Photo p={p} />
            <span className="ta-list-text">
              <span className="ta-small-name">{p.name}</span>
              <span className="ta-small-meta">
                {p.country}, <Star className="ta-ico-s ta-star" /> {p.rating}
              </span>
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

/** Foldable, inner right panel: the selected destination in detail. */
export function FoldDetailScreen() {
  const p = places[0];
  return (
    <div className="ta ta-fold-detail">
      <StatusBar />
      <div className="ta-detail-photo">
        <Photo p={p} pos="50% 55%" />
      </div>
      <span className="ta-detail-name">{p.name}</span>
      <span className="ta-feature-meta ta-detail-meta">
        <MapPin className="ta-ico-s" />
        {p.country}
        <Star className="ta-ico-s ta-star" />
        {p.rating}
      </span>
      <p className="ta-detail-copy">Tram 28 climbs through Alfama to the viewpoints above the river.</p>
      <span className="ta-cta">Plan a trip</span>
    </div>
  );
}

/** Foldable, outer (cover) display, seen once the device has closed. */
export function CoverScreen() {
  return (
    <div className="ta ta-cover">
      <StatusBar />
      <Header compact />
      <Feature p={places[0]} tall />
      <BottomNav />
    </div>
  );
}
