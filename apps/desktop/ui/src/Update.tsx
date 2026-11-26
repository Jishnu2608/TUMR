import { useCallback, useEffect, useState } from "react";
import { applyUpdate, checkUpdate, notifyUpdate, openUpdateLink, UpdateInfo } from "./api";

/**
 * UPD-004: "a new version is available" as a toast, in the main window and in the phone window.
 *
 * Three ways to update, the user picks:
 * - **Update now**: `tumr update --yes --relaunch` (download, size + SHA-256 check, install over
 *   this copy; settings, toolchains and emulators stay; the app opens again).
 * - **From the terminal**: copies `tumr update` and shows the line to paste in PowerShell.
 * - **Download the installer**: opens the release's installer in the browser.
 *
 * **Later** hides the toast until a newer version than the one dismissed (remembered per version,
 * shared by both windows). The channel is checked at start and again every few hours while the app
 * stays open; with `[update] manifest_url = "off"` nothing is contacted and nothing shows.
 *
 * When no window of the app is in front (minimised, behind other programs), one Windows
 * notification per version says so. The main window and the phone window are separate processes,
 * so they agree through `localStorage`: a window in front marks the version as seen; a window in
 * the background waits a moment and notifies only if no window has seen it yet.
 */

/** How often an open app checks again. */
const RECHECK_MS = 8 * 60 * 60 * 1000;
/** The version the user said "Later" to. */
const LATER_KEY = "tumr.update.later";
const COMMAND = "tumr update";
/** The version a toast was seen for, or a notification was shown for. */
const SEEN_KEY = "tumr.update.seen";
/** How long a background window waits for a window in front to see the toast. */
const NOTIFY_WAIT_MS = 3000;

function readKey(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeKey(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* storage off: at worst a second notification */
  }
}

/** Marks the version seen when this window is in front, else notifies once (see above). */
function announce(version: string): () => void {
  if (readKey(SEEN_KEY) === version) return () => undefined;
  const inFront = () => document.visibilityState === "visible" && document.hasFocus();
  if (inFront()) {
    writeKey(SEEN_KEY, version);
    return () => undefined;
  }
  const t = window.setTimeout(() => {
    if (readKey(SEEN_KEY) === version) return;
    writeKey(SEEN_KEY, version);
    if (!inFront()) notifyUpdate().catch(() => undefined);
  }, NOTIFY_WAIT_MS);
  // coming to the front before the wait ends counts as seen
  const onFocus = () => writeKey(SEEN_KEY, version);
  window.addEventListener("focus", onFocus, { once: true });
  return () => {
    window.clearTimeout(t);
    window.removeEventListener("focus", onFocus);
  };
}

function laterVersion(): string | null {
  try {
    return localStorage.getItem(LATER_KEY);
  } catch {
    return null;
  }
}

function rememberLater(version: string) {
  try {
    localStorage.setItem(LATER_KEY, version);
  } catch {
    /* private storage off: the toast just comes back at the next check */
  }
}

export function UpdateToast({ compact = false }: { compact?: boolean }) {
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [later, setLater] = useState<string | null>(laterVersion);
  const [state, setState] = useState<"idle" | "updating" | "terminal">("idle");
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const check = useCallback(() => {
    checkUpdate()
      .then((i) => {
        setInfo(i);
        setLater(laterVersion());
      })
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    check();
    const id = window.setInterval(check, RECHECK_MS);
    // the other window said "Later": hide here too
    const onStorage = (e: StorageEvent) => e.key === LATER_KEY && setLater(e.newValue);
    window.addEventListener("storage", onStorage);
    return () => {
      window.clearInterval(id);
      window.removeEventListener("storage", onStorage);
    };
  }, [check]);

  const next = info?.available;
  const wanted = next && later !== next.version ? next.version : null;
  useEffect(() => (wanted ? announce(wanted) : undefined), [wanted]);
  if (!next || later === next.version) return null;

  const fail = (e: unknown) => setError(String(e).replace(/^Error:\s*/, ""));
  const copy = async () => {
    setState("terminal");
    setError(null);
    try {
      await navigator.clipboard.writeText(COMMAND);
      setCopied(true);
    } catch {
      setCopied(false); // the line stays on screen to copy by hand
    }
  };
  // the browser demo has no file list; a real release lists its installer
  const hasInstaller = next.files ? next.files.some((f) => f.kind === "installer") : true;

  return (
    <div className={compact ? "upd-toast compact" : "upd-toast"} role="status" aria-live="polite">
      <p className="upd-title">
        <strong>Version {next.version} is available</strong> <span className="upd-muted">(you have {info?.current})</span>
      </p>
      {next.notes_url && (
        <button className="link upd-notes" onClick={() => openUpdateLink("notes").catch(fail)}>
          What's new
        </button>
      )}
      {state === "updating" ? (
        <p className="upd-muted">Downloading and installing… the app closes and opens again by itself.</p>
      ) : (
        <>
          {state === "terminal" && (
            <div className="upd-terminal">
              <p>{copied ? "Copied. Paste it in PowerShell and press Enter:" : "Run this in PowerShell:"}</p>
              <code>{COMMAND}</code>
            </div>
          )}
          <div className="upd-actions">
            <button
              className="primary"
              onClick={() => {
                setState("updating");
                setError(null);
                applyUpdate().catch((e) => {
                  fail(e);
                  setState("idle");
                });
              }}
            >
              Update now
            </button>
            <button className="secondary" onClick={copy}>
              Update from the terminal
            </button>
            {hasInstaller && (
              <button className="secondary" onClick={() => openUpdateLink("installer").catch(fail)}>
                Download the installer
              </button>
            )}
            <button
              className="link"
              onClick={() => {
                rememberLater(next.version);
                setLater(next.version);
              }}
            >
              Later
            </button>
          </div>
        </>
      )}
      {error && <p className="upd-error">{error}</p>}
    </div>
  );
}
