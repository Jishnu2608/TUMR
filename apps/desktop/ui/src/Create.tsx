// CRT-E: the Create Project wizard (TUMR Create in the desktop app). Same steps and fields as
// `tumr create` in the CLI, on the same engine: the Runtime Core's `project.create.*` job makes
// the project; the agent is connected and opened through `tumr agent connect|open --json`
// (`agent_cli`), so the CLI and the app share one implementation. Nothing is written before the
// Create step; the framework's tool runs in a staging folder and rolls back on failure or cancel.
import { useEffect, useRef, useState } from "react";
import { CircleCheck, CircleHelp, CircleX, FolderOpen, FolderPlus, Hand, LoaderCircle } from "lucide-react";
import { agentCli, call, openDocs, pickFolder } from "./api";
import { Busy, InstallStep, LicenceStep, Nav, Problem } from "./App";

type CStep = "framework" | "details" | "place" | "agent" | "check" | "create" | "done";

/** The guide section for each step (docs/user/create-project.md headings). */
const DOCS_ANCHOR: Record<CStep, string | undefined> = {
  framework: "the-frameworks",
  details: "name-package-id-and-folder",
  place: "name-package-id-and-folder",
  agent: "your-coding-agent",
  check: "prerequisites",
  create: "troubleshooting",
  done: "opening-the-agent",
};

const STEPS: { id: CStep; label: string }[] = [
  { id: "framework", label: "Framework" },
  { id: "details", label: "Name" },
  { id: "place", label: "Location" },
  { id: "agent", label: "Agent" },
  { id: "check", label: "Check" },
  { id: "create", label: "Create" },
  { id: "done", label: "Done" },
];

interface Framework {
  id: string;
  name: string;
  description: string;
  language: string;
  creation_tools: string[];
  available: boolean;
}

interface Choice {
  framework: Framework | null;
  name: string;
  packageId: string;
  /** typed by the user (otherwise the Core's suggestion follows the name) */
  packageEdited: boolean;
  mode: "new" | "empty";
  parent: string;
  folder: string;
  agent: string;
  trust: boolean;
}

/** The five agents TUMR Create offers: `id` is recorded in the project (as `tumr create --agent`
 * does, CRT-F parity), `client` is its entry in `tumr agent clients` (config files, setup). */
const AGENTS: { id: string; client: string; name: string }[] = [
  { id: "claude-code", client: "claude-code", name: "Claude Code" },
  { id: "codex", client: "codex", name: "Codex" },
  { id: "cursor", client: "cursor", name: "Cursor" },
  { id: "antigravity", client: "antigravity", name: "Google Antigravity" },
  { id: "copilot", client: "vscode", name: "GitHub Copilot (VS Code)" },
];

const STATE_LABEL: Record<string, string> = {
  ready: "Installed, connected",
  "not-configured": "Installed, not connected yet",
  "not-installed": "Not installed",
  unsupported: "Found, cannot be started",
};

function request(c: Choice, withPackage = true) {
  return {
    framework: c.framework?.id,
    name: c.name.trim(),
    package_id: withPackage && c.packageEdited && c.packageId.trim() ? c.packageId.trim() : undefined,
    destination: c.mode === "new" ? { mode: "new-in", parent: c.parent } : { mode: "existing-empty", path: c.folder },
    agent: AGENTS.find((a) => a.client === c.agent)?.id,
    trust: c.trust,
  };
}

/** The same rules as the engine (`tumr_create::request::check_name`), for typing feedback. */
function nameProblem(n: string): string | null {
  const t = n.trim();
  if (!t) return "Type a name.";
  if (t.length > 64) return "At most 64 characters.";
  if (!/^[A-Za-z]/.test(t)) return "Start with a letter (A–Z).";
  if (!/^[A-Za-z][A-Za-z0-9 _-]*$/.test(t)) return "Use letters, digits, spaces, - and _ only.";
  return null;
}

function packageProblem(p: string): string | null {
  const t = p.trim();
  if (!t) return null;
  const parts = t.split(".");
  if (parts.length < 2) return "At least two parts, such as com.example.app.";
  const bad = parts.find((s) => !/^[A-Za-z][A-Za-z0-9_]*$/.test(s));
  return bad !== undefined ? `Each part starts with a letter and uses letters, digits and _ ('${bad}').` : null;
}

function errorText(e: unknown): string {
  return String(e).replace(/^Error:\s*/, "");
}

export function CreateWizard({ onClose, onRun, onCheck }: { onClose: () => void; onRun: (path: string) => void; onCheck: (path: string) => void }) {
  const [step, setStep] = useState<CStep>("framework");
  const [c, setC] = useState<Choice>({ framework: null, name: "", packageId: "", packageEdited: false, mode: "new", parent: "", folder: "", agent: "", trust: true });
  const [result, setResult] = useState<any>(null);
  const set = (p: Partial<Choice>) => setC((o) => ({ ...o, ...p }));
  const index = STEPS.findIndex((s) => s.id === step);
  // after the project exists, the earlier steps cannot be changed any more
  const locked = index >= STEPS.findIndex((s) => s.id === "create");
  return (
    <>
      <ol className="steps" aria-label="Create project steps">
        {STEPS.map((s, i) => (
          <li key={s.id} className={i < index ? "done" : i === index ? "current" : ""} aria-current={i === index ? "step" : undefined}>
            {i < index && !locked ? (
              <button type="button" className="step-back" onClick={() => setStep(s.id)} aria-label={`Back to ${s.label}`}>
                <span className="dot">✓</span>
                {s.label}
              </button>
            ) : (
              <>
                <span className="dot">{i < index ? "✓" : i + 1}</span>
                {s.label}
              </>
            )}
          </li>
        ))}
      </ol>
      {step === "framework" && <FrameworkStep value={c.framework} onPick={(f) => set({ framework: f })} onBack={onClose} onNext={() => setStep("details")} />}
      {step === "details" && <DetailsStep c={c} set={set} onBack={() => setStep("framework")} onNext={() => setStep("place")} />}
      {step === "place" && <PlaceStep c={c} set={set} onBack={() => setStep("details")} onNext={() => setStep("agent")} />}
      {step === "agent" && <AgentStep c={c} set={set} onBack={() => setStep("place")} onNext={() => setStep("check")} />}
      {step === "check" && <CheckStep c={c} onBack={() => setStep("agent")} onNext={() => setStep("create")} />}
      {step === "create" && (
        <CreateStep
          c={c}
          onBack={() => setStep("check")}
          onDone={(st) => {
            setResult(st);
            setStep("done");
          }}
        />
      )}
      {step === "done" && result && <DoneStep c={c} status={result} onClose={onClose} onRun={onRun} onCheck={onCheck} />}
      <p className="step-docs small">
        <button className="link" onClick={() => openDocs("create-project", DOCS_ANCHOR[step]).catch(() => undefined)}>
          {step === "agent" || step === "done" ? "Agents, connecting and opening" : step === "place" ? "Name, package id and folder" : "Creating a project"} (guide) ↗
        </button>
      </p>
    </>
  );
}

function FrameworkStep({ value, onPick, onBack, onNext }: { value: Framework | null; onPick: (f: Framework) => void; onBack: () => void; onNext: () => void }) {
  const [list, setList] = useState<Framework[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    call<{ frameworks: Framework[] }>("project.create.frameworks")
      .then((v) => setList(v.frameworks))
      .catch((e) => setError(errorText(e)));
  }, []);
  if (error) return <section><Problem text={error} /></section>;
  if (!list) return <Busy text="Loading the frameworks…" />;
  return (
    <section>
      <h2>Create a new project</h2>
      <p className="muted">
        TUMR makes a minimal, empty project with the framework's official tool (no sample app) and checks that it is recognised. Your coding agent builds the app.
      </p>
      <div className="choices" role="radiogroup" aria-label="Framework">
        {list.map((f) => (
          <button
            key={f.id}
            type="button"
            role="radio"
            aria-checked={value?.id === f.id}
            aria-label={`${f.name}: ${f.description}`}
            className={value?.id === f.id ? "choice picked" : "choice"}
            onClick={() => onPick(f)}
            onDoubleClick={() => {
              onPick(f);
              onNext();
            }}
          >
            <span className="choice-title">{f.name}</span>
            <span className="small muted">{f.description}</span>
          </button>
        ))}
      </div>
      <Nav onBack={onBack} backLabel="Cancel" onNext={onNext} nextDisabled={!value} nextLabel="Next" />
    </section>
  );
}

/** Asks the Core whether the request would be accepted (no writing); debounced. */
function useCheck(c: Choice, ready: boolean) {
  const [v, setV] = useState<{ target: string; package_id: string; warnings: string[] } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const key = JSON.stringify(request(c));
  useEffect(() => {
    if (!ready) {
      setV(null);
      setError(null);
      return;
    }
    let stop = false;
    setBusy(true);
    const t = window.setTimeout(() => {
      call("project.create.check", request(c))
        .then((r) => {
          if (stop) return;
          setV(r);
          setError(null);
        })
        .catch((e) => {
          if (stop) return;
          setV(null);
          setError(errorText(e));
        })
        .finally(() => !stop && setBusy(false));
    }, 300);
    return () => {
      stop = true;
      window.clearTimeout(t);
    };
  }, [key, ready]); // eslint-disable-line react-hooks/exhaustive-deps
  return { v, error, busy };
}

function DetailsStep({ c, set, onBack, onNext }: { c: Choice; set: (p: Partial<Choice>) => void; onBack: () => void; onNext: () => void }) {
  const nameErr = nameProblem(c.name);
  const pkgErr = c.packageEdited ? packageProblem(c.packageId) : null;
  // the suggestion comes from the engine (package id from the name); the folder is not chosen yet
  const probe: Choice = { ...c, mode: "new", parent: c.parent || "C:\\" };
  const { v, error } = useCheck(probe, !nameErr && !pkgErr);
  const suggested = v?.package_id ?? "";
  const engineErr = error && !/folder|exists|empty|path/i.test(error) ? error : null;
  return (
    <section>
      <h2>Name and package id</h2>
      <label htmlFor="cname">Project name</label>
      <input id="cname" value={c.name} onChange={(e) => set({ name: e.target.value })} placeholder="My App" autoFocus aria-invalid={!!c.name && !!nameErr} aria-describedby="cname-help" />
      <p id="cname-help" className={c.name && nameErr ? "field-error small" : "small muted"}>
        {c.name && nameErr ? nameErr : "Also the folder name when TUMR makes a new folder."}
      </p>
      <label htmlFor="cpkg">Android package id</label>
      <input
        id="cpkg"
        value={c.packageEdited ? c.packageId : suggested}
        placeholder={suggested || "com.example.myapp"}
        onChange={(e) => set({ packageId: e.target.value, packageEdited: e.target.value.trim() !== "" })}
        aria-invalid={!!pkgErr || !!engineErr}
        aria-describedby="cpkg-help"
      />
      <p id="cpkg-help" className={pkgErr || engineErr ? "field-error small" : "small muted"}>
        {pkgErr ?? engineErr ?? "The app's id on the device and in stores. Suggested from the name; type to change it."}
      </p>
      <Nav onBack={onBack} onNext={onNext} nextDisabled={!!nameErr || !!pkgErr || !!engineErr} nextLabel="Next" />
    </section>
  );
}

function PlaceStep({ c, set, onBack, onNext }: { c: Choice; set: (p: Partial<Choice>) => void; onBack: () => void; onNext: () => void }) {
  const chosen = c.mode === "new" ? c.parent : c.folder;
  const { v, error, busy } = useCheck(c, !!chosen);
  const [pickError, setPickError] = useState<string | null>(null);
  const pick = async () => {
    setPickError(null);
    const p = await pickFolder(chosen || undefined, c.mode === "new" ? "parent" : "empty").catch((e) => {
      setPickError(errorText(e));
      return null;
    });
    if (p) set(c.mode === "new" ? { parent: p } : { folder: p });
  };
  return (
    <section>
      <h2>Where the project goes</h2>
      <div className="choices two" role="radiogroup" aria-label="Destination">
        <button type="button" role="radio" aria-label="A new folder inside a folder you choose" aria-checked={c.mode === "new"} className={c.mode === "new" ? "choice picked" : "choice"} onClick={() => set({ mode: "new" })}>
          <span className="choice-title">
            <FolderPlus aria-hidden /> A new folder
          </span>
          <span className="small muted">TUMR makes “{c.name.trim() || "the project name"}” inside a folder you choose.</span>
        </button>
        <button type="button" role="radio" aria-label="An existing, empty folder" aria-checked={c.mode === "empty"} className={c.mode === "empty" ? "choice picked" : "choice"} onClick={() => set({ mode: "empty" })}>
          <span className="choice-title">
            <FolderOpen aria-hidden /> An existing, empty folder
          </span>
          <span className="small muted">The folder you choose becomes the project. It must be empty.</span>
        </button>
      </div>
      <div className="picker">
        <button className="primary" onClick={pick}>
          {chosen ? "Choose another folder…" : c.mode === "new" ? "Choose the parent folder…" : "Choose the empty folder…"}
        </button>
        {chosen && (
          <span className="picked" title={chosen}>
            📁 <code>{chosen}</code>
          </span>
        )}
      </div>
      <p className="small">
        <label className="inline" htmlFor="cpath">
          Or type the path
        </label>
        <input
          id="cpath"
          value={chosen}
          placeholder={c.mode === "new" ? "D:\\Work" : "D:\\Work\\My App"}
          onChange={(e) => set(c.mode === "new" ? { parent: e.target.value.replace(/^"|"$/g, "") } : { folder: e.target.value.replace(/^"|"$/g, "") })}
        />
      </p>
      {pickError && <div className="problem" role="alert">{pickError}</div>}
      {chosen && busy && <p className="small muted">Checking the folder…</p>}
      {chosen && !busy && error && (
        <div className="problem" role="alert">
          {error}
        </div>
      )}
      {chosen && !busy && v && (
        <div className="card">
          <p className="big">
            <CircleCheck aria-hidden className="ok-icon" /> The project will be <code>{v.target}</code>
          </p>
          {v.warnings.map((w, i) => (
            <p key={i} className="small warn">
              {w}
            </p>
          ))}
        </div>
      )}
      <Nav onBack={onBack} onNext={onNext} nextDisabled={!v || busy || !!error} nextLabel="Next" />
    </section>
  );
}

interface ClientRow {
  id: string;
  name: string;
  state?: string;
  state_label?: string;
  installed: boolean;
}

function AgentStep({ c, set, onBack, onNext }: { c: Choice; set: (p: Partial<Choice>) => void; onBack: () => void; onNext: () => void }) {
  const [rows, setRows] = useState<ClientRow[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const project = c.mode === "new" ? c.parent : c.folder;
  useEffect(() => {
    agentCli<ClientRow[]>({ action: "clients", project })
      .then(setRows)
      .catch((e) => setError(errorText(e)));
  }, [project]);
  const stateOf = (id: string) => rows?.find((r) => r.id === id)?.state ?? (rows?.find((r) => r.id === id)?.installed ? "not-configured" : "not-installed");
  return (
    <section>
      <h2>Coding agent</h2>
      <p className="muted">TUMR connects the agent to this project and can open it there. You can also choose later.</p>
      {error && <Problem text={error} />}
      {!rows && !error && <Busy text="Looking for coding agents on this PC…" />}
      {rows && (
        <div className="choices" role="radiogroup" aria-label="Coding agent">
          {AGENTS.map((a) => {
            const s = stateOf(a.client);
            return (
              <button key={a.client} type="button" role="radio" aria-label={`${a.name}: ${STATE_LABEL[s] ?? s}`} aria-checked={c.agent === a.client} className={c.agent === a.client ? "choice picked" : "choice"} onClick={() => set({ agent: a.client })}>
                <span className="choice-title">{a.name}</span>
                <span className={`badge ${s}`}>{STATE_LABEL[s] ?? s}</span>
              </button>
            );
          })}
          <button type="button" role="radio" aria-label="No agent for now" aria-checked={c.agent === ""} className={c.agent === "" ? "choice picked" : "choice"} onClick={() => set({ agent: "" })}>
            <span className="choice-title">None for now</span>
            <span className="small muted">Connect one later on the Agent page.</span>
          </button>
        </div>
      )}
      <label className="check trust-check">
        <input type="checkbox" checked={c.trust} onChange={(e) => set({ trust: e.target.checked })} /> Trust the new project, so TUMR may build it (this folder only)
      </label>
      <Nav onBack={onBack} onNext={onNext} nextDisabled={!rows} nextLabel="Check the environment" />
    </section>
  );
}

function CheckStep({ c, onBack, onNext }: { c: Choice; onBack: () => void; onNext: () => void }) {
  const [plan, setPlan] = useState<any>(null);
  const [error, setError] = useState<string | null>(null);
  const [target, setTarget] = useState<string>("");
  const [phase, setPhase] = useState<"check" | "licences" | "install">("check");
  const [jobId, setJobId] = useState<string | undefined>();
  const [round, setRound] = useState(0);
  const fw = c.framework!;
  useEffect(() => {
    setPlan(null);
    setError(null);
    call("project.create.check", request(c))
      .then((v) => {
        setTarget(v.target);
        return call("project.plan", { path: v.target, framework: fw.id });
      })
      .then((v) => setPlan(v.plan))
      .catch((e) => setError(errorText(e)));
  }, [round]); // eslint-disable-line react-hooks/exhaustive-deps
  if (phase === "licences") return <LicenceStep path={target} framework={fw.id} onBack={() => setPhase("check")} onNext={() => setPhase("install")} />;
  if (phase === "install")
    return (
      <InstallStep
        path={target}
        framework={fw.id}
        jobId={jobId}
        onJob={setJobId}
        onBack={() => setPhase("check")}
        onDone={() => {
          setJobId(undefined);
          setPhase("check");
          setRound((r) => r + 1);
        }}
      />
    );
  if (error) return <section><Problem text={error} retry={() => setRound((r) => r + 1)} /><Nav onBack={onBack} /></section>;
  if (!plan) return <Busy text="Checking the tools for this framework…" />;
  const tools = plan.items.filter((i: any) => fw.creation_tools.includes(i.component));
  const missing = tools.filter((i: any) => i.action.kind === "install");
  const later = plan.items.filter((i: any) => !fw.creation_tools.includes(i.component) && i.action.kind === "install");
  return (
    <section>
      <h2>Environment check</h2>
      {fw.creation_tools.length === 0 && <p>{fw.name} is created from TUMR's own template: no tool is needed to create it.</p>}
      {tools.length > 0 && (
        <ul className="tool-list">
          {tools.map((i: any) => (
            <li key={i.component}>
              {i.action.kind === "install" ? <CircleX aria-hidden className="bad-icon" /> : <CircleCheck aria-hidden className="ok-icon" />}
              <span>
                {i.title} {i.version && <span className="muted">{i.version}</span>}
              </span>
              <span className={i.action.kind === "install" ? "badge unsupported" : "badge ready"}>{i.action.kind === "install" ? "Missing" : "Found"}</span>
            </li>
          ))}
        </ul>
      )}
      {later.length > 0 && (
        <p className="small muted">
          To build and run it later TUMR also needs {later.map((i: any) => i.title).join(", ")}. Run offers to install them.
        </p>
      )}
      {missing.length > 0 ? (
        <>
          <p className="warn">
            {fw.name} needs {missing.map((i: any) => i.title).join(", ")} before the project can be created. TUMR installs it into its own folder (no global
            changes), after you accept its licence.
          </p>
          <Nav onBack={onBack} onNext={() => setPhase("licences")} nextLabel={`Install ${missing.length === 1 ? missing[0].title : "them"}`} />
        </>
      ) : (
        <Nav onBack={onBack} onNext={onNext} nextLabel="Review and create" />
      )}
    </section>
  );
}

function CreateStep({ c, onBack, onDone }: { c: Choice; onBack: () => void; onDone: (status: any) => void }) {
  const [status, setStatus] = useState<any>(null);
  const [error, setError] = useState<string | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const live = useRef(true);
  useEffect(() => {
    // StrictMode mounts twice in development: set it on every mount, not only at creation
    live.current = true;
    return () => {
      live.current = false;
    };
  }, []);
  const agent = AGENTS.find((a) => a.client === c.agent);
  const { v: checked } = useCheck(c, true);
  const start = async () => {
    setError(null);
    try {
      const r = await call("project.create.start", request(c));
      for (;;) {
        const st = await call("project.create.status", { job_id: r.job_id });
        if (!live.current) return;
        setStatus(st);
        if (st.state !== "running") {
          if (st.state === "done") onDone(st);
          return;
        }
        await new Promise((res) => setTimeout(res, 400));
      }
    } catch (e) {
      setError(errorText(e));
    }
  };
  const cancel = () => {
    setCancelling(true);
    call("project.create.cancel", { job_id: status?.id }).catch(() => undefined);
  };
  if (!status)
    return (
      <section>
        <h2>Review and create</h2>
        <table className="plan summary">
          <tbody>
            <tr><td>Framework</td><td>{c.framework!.name} <span className="muted small">· {c.framework!.description}</span></td></tr>
            <tr><td>Name</td><td>{c.name.trim()}</td></tr>
            <tr><td>Package id</td><td><code>{checked?.package_id ?? (c.packageEdited ? c.packageId.trim() : "…")}</code></td></tr>
            <tr><td>Folder</td><td><code>{checked?.target ?? "…"}</code> <span className="muted small">({c.mode === "new" ? "new" : "existing, empty"})</span></td></tr>
            <tr><td>Agent</td><td>{agent?.name ?? "None"}</td></tr>
            <tr><td>Trust</td><td>{c.trust ? "Yes, this folder only" : "No"}</td></tr>
          </tbody>
        </table>
        {error && <Problem text={error} />}
        <Nav onBack={onBack} onNext={start} nextLabel="Create the project" />
      </section>
    );
  const failed = status.state === "failed" || status.state === "cancelled";
  return (
    <section>
      <h2>{status.state === "running" ? `Creating ${status.name}…` : status.state === "cancelled" ? "Cancelled" : "The creation failed"}</h2>
      <ol className="create-log" aria-live="polite">
        {status.progress.map((p: any, i: number) => (
          <li key={i}>
            <span className="tag reuse">{p.stage}</span> {p.message}
          </li>
        ))}
        {status.state === "running" && (
          <li className="muted">
            <LoaderCircle aria-hidden className="spin" /> working…
          </li>
        )}
      </ol>
      {status.state === "running" && (
        <button className="secondary" onClick={cancel} disabled={cancelling}>
          {cancelling ? "Cancelling…" : "Cancel (removes what was made)"}
        </button>
      )}
      {failed && status.error && (
        <div className={status.state === "cancelled" ? "note" : "problem"} role="alert">
          {status.state !== "cancelled" && (
            <p>
              <strong>{status.error.stage}:</strong> {status.error.message}
            </p>
          )}
          <p className="small">{status.error.cleanup}</p>
        </div>
      )}
      {failed && (
        <Nav
          onBack={() => {
            setStatus(null);
            setCancelling(false);
            onBack();
          }}
          backLabel="Back to the check"
          onNext={() => {
            setStatus(null);
            setCancelling(false);
          }}
          nextLabel="Try again"
        />
      )}
    </section>
  );
}

interface OpenStep {
  step: string;
  label: string;
  status: "verified" | "not-verified" | "manual" | "failed";
  detail: string;
}

function StepIcon({ s }: { s: OpenStep["status"] }) {
  if (s === "verified") return <CircleCheck aria-label="TUMR saw it" className="ok-icon" />;
  if (s === "failed") return <CircleX aria-label="failed" className="bad-icon" />;
  if (s === "manual") return <Hand aria-label="a step for you" className="warn-icon" />;
  return <CircleHelp aria-label="TUMR could not see it" className="muted-icon" />;
}

function DoneStep({ c, status, onClose, onRun, onCheck }: { c: Choice; status: any; onClose: () => void; onRun: (p: string) => void; onCheck: (p: string) => void }) {
  const r = status.result;
  const [trusted, setTrusted] = useState<boolean>(status.trusted);
  const [trustError, setTrustError] = useState<string | null>(null);
  const agent = AGENTS.find((a) => a.client === c.agent);
  const confirmTrust = () =>
    call("project.trust", { path: r.path, trusted: true })
      .then(() => setTrusted(true))
      .catch((e) => setTrustError(errorText(e)));
  return (
    <section>
      <h2>
        <CircleCheck aria-hidden className="ok-icon" /> {c.framework!.name} project “{c.name.trim()}” created
      </h2>
      <p>
        <code>{r.path}</code>
      </p>
      <p className="small muted">
        Recognised by TUMR ({r.confidence} % sure) · package id <code>{r.package_id}</code> · {trusted ? "trusted" : "not trusted"}
      </p>
      {r.warnings.map((w: string, i: number) => (
        <p key={i} className="small warn">
          {w}
        </p>
      ))}
      {status.trust_needs_confirmation && !trusted && (
        <div className="confirm" role="alertdialog" aria-label="Unexpected files">
          <p>
            <strong>Not trusted yet.</strong> The project holds {r.unexpected_count} file(s) the {c.framework!.name} creator does not normally make. Check
            them before you trust it (its build scripts may then run):
          </p>
          <ul className="small">
            {r.unexpected.map((f: string) => (
              <li key={f}>
                <code>{f}</code>
              </li>
            ))}
          </ul>
          {trustError && <p className="problem small">{trustError}</p>}
          <div className="row">
            <span />
            <button onClick={confirmTrust}>Trust it anyway</button>
          </div>
        </div>
      )}
      {agent && <AgentPanel agent={agent} project={r.path} />}
      <div className="nav">
        <button className="secondary" onClick={onClose}>
          Done
        </button>
        <span>
          <button className="secondary" onClick={() => onCheck(r.path)}>
            See what running needs
          </button>{" "}
          <button onClick={() => onRun(r.path)}>
            Run it
          </button>
        </span>
      </div>
    </section>
  );
}

/** CRT-D5 connection (shown, then applied, backed up) and the CRT-D launch with what TUMR saw. */
function AgentPanel({ agent, project }: { agent: { id: string; client: string; name: string }; project: string }) {
  const [plan, setPlan] = useState<any>(null);
  const [error, setError] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);
  const [applied, setApplied] = useState<any>(null);
  const [state, setState] = useState<string | null>(null);
  const [opening, setOpening] = useState(false);
  const [steps, setSteps] = useState<OpenStep[] | null>(null);
  useEffect(() => {
    agentCli<ClientRow[]>({ action: "clients", project })
      .then((rows) => setState(rows.find((x) => x.id === agent.client)?.state ?? null))
      .catch(() => setState(null));
    agentCli({ action: "connect_plan", client: agent.client, project })
      .then(setPlan)
      .catch((e) => setError(errorText(e)));
  }, [agent.client, project]);
  const apply = () => {
    setApplying(true);
    setError(null);
    agentCli({ action: "connect_apply", client: agent.client, project })
      .then(setApplied)
      .catch((e) => setError(errorText(e)))
      .finally(() => setApplying(false));
  };
  const open = () => {
    setOpening(true);
    setError(null);
    agentCli<{ steps: OpenStep[] }>({ action: "open", client: agent.client, project })
      .then((v) => setSteps(v.steps))
      .catch((e) => setError(errorText(e)))
      .finally(() => setOpening(false));
  };
  const usable = state !== "not-installed" && state !== "unsupported";
  const connected = applied || plan?.up_to_date;
  return (
    <div className="card">
      <h3>{agent.name}</h3>
      {!usable && state && <p className="warn">{agent.name} is {STATE_LABEL[state]?.toLowerCase()} on this PC. TUMR recorded it for the project; install it to connect.</p>}
      {error && <div className="problem" role="alert">{error}</div>}
      {usable && !plan && !error && <Busy text="Working out what connecting changes…" />}
      {usable && plan && !connected && (
        <>
          <p className="small">
            Connecting writes {plan.changes.length} file(s){plan.user_connected_before ? " in this project" : " (the agent's user settings once, plus this project's instructions)"}. Other servers and
            settings are kept; changed files are backed up first.
          </p>
          <ul className="changes">
            {plan.changes.map((ch: any) => (
              <li key={ch.path}>
                <details>
                  <summary>
                    <span className="tag install">{ch.action}</span> <code>{ch.path}</code>
                  </summary>
                  <pre className="diff">{ch.diff}</pre>
                </details>
              </li>
            ))}
          </ul>
          <button onClick={apply} disabled={applying}>
            {applying ? "Connecting…" : `Connect ${agent.name}`}
          </button>
        </>
      )}
      {usable && connected && (
        <p className="ok">
          Connected{applied?.backup ? <span className="muted small"> · backup in <code>{applied.backup}</code></span> : null}
        </p>
      )}
      {usable && connected && !steps && (
        <button onClick={open} disabled={opening}>
          {opening ? `Opening ${agent.name}… (up to a minute)` : `Open ${agent.name} in the project`}
        </button>
      )}
      {steps && (
        <>
          <ul className="open-steps">
            {steps.map((s) => (
              <li key={s.step}>
                <StepIcon s={s.status} />
                <strong>{s.label}</strong>
                <span className="small muted">{s.detail}</span>
              </li>
            ))}
          </ul>
          <p className="small muted">Only what TUMR could see is marked done; a question mark means it may have happened, or not yet.</p>
        </>
      )}
    </div>
  );
}
