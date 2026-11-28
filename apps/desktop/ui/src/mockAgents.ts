// Sample answers for the "Connect your agent" page in a plain browser (UI development,
// screenshots). Shapes match `tumr agent clients|setup|doctor --json`.
let pathOn = false;
const connected: Record<string, boolean> = { "claude-code": true };
const userScoped = new Set(["codex", "claude-desktop", "antigravity"]);
const project = String.raw`D:\Work\my-app`;

const clients: [string, string, boolean][] = [
  ["claude-code", "Claude Code", true],
  ["codex", "Codex", false],
  ["cursor", "Cursor", true],
  ["claude-desktop", "Claude Desktop", true],
  ["vscode", "VS Code (agent mode)", true],
  ["antigravity", "Antigravity", true],
];

const entryDiff = (id: string) =>
  [
    "+ {",
    '+   "mcpServers": {',
    '+     "tumr": {',
    '+       "type": "stdio",',
    String.raw`+       "command": "C:\\Users\\you\\AppData\\Local\\Toolchain Universal Mobile Runtime\\tumr.exe",`,
    `+       "args": ["mcp", "--client", "${id}"]`,
    "+     }",
    "+   }",
    "+ }",
  ].join("\n");

export async function mockAgent(req: { action: string; client?: string; remove?: boolean; scope?: string }): Promise<unknown> {
  await new Promise((r) => setTimeout(r, 300));
  const id = req.client ?? "";
  if (req.action.startsWith("path_")) {
    if (req.action !== "path_status") pathOn = req.action === "path_add";
    return { on: pathOn, changed: req.action !== "path_status", dir: String.raw`C:\Users\you\AppData\Local\Toolchain Universal Mobile Runtime`, chosen: pathOn };
  }
  if (req.action === "clients") {
    return clients.map(([cid, name, installed]) => ({
      id: cid,
      name,
      installed,
      state: !installed ? "not-installed" : connected[cid] ? "ready" : "not-configured",
      project_connected: !!connected[cid] && !userScoped.has(cid),
      user_connected: !!connected[cid] && userScoped.has(cid),
      project_config: cid === "claude-desktop" ? null : `${project}\\.config`,
      user_config: String.raw`C:\Users\you\.config`,
    }));
  }
  // CRT-E: `tumr agent connect` and `tumr agent open` (the Create Project wizard)
  if (req.action === "connect_plan" || req.action === "connect_apply") {
    const apply = req.action === "connect_apply";
    if (apply) connected[id] = true;
    return {
      client: id,
      name: id,
      user_connected_before: userScoped.has(id),
      client_exists: !!connected[id],
      up_to_date: false,
      applied: apply,
      backup: apply ? String.raw`C:\Users\you\.tumr\backups\agent-config\1795820000-1234` : null,
      changes: [
        ...(userScoped.has(id) ? [] : [{ path: String.raw`C:\Users\you\.cursor\mcp.json`, action: "change", diff: entryDiff(id) }]),
        { path: String.raw`D:\Work\My Apps\My App\.cursor\rules\tumr-runtime.mdc`, action: "create", diff: "+ ---\n+ description: Run and check the user's Android app …\n+ alwaysApply: true\n+ ---" },
      ],
    };
  }
  if (req.action === "open") {
    await new Promise((r) => setTimeout(r, 1200));
    const codex = id === "codex";
    return {
      agent: id,
      launched: true,
      steps: [
        { step: "mcp-config", label: "MCP config written", status: "verified", detail: String.raw`C:\Users\you\.cursor\mcp.json starts tumr` },
        { step: "app-launched", label: "app launched", status: "verified", detail: codex ? "the Codex app runs (ChatGPT.exe)" : "Cursor runs (Cursor.exe)" },
        codex
          ? { step: "workspace-opened", label: "workspace opened", status: "manual", detail: "the Codex app has no supported way to be given a folder: in the app, open the project" }
          : { step: "workspace-opened", label: "workspace opened", status: "verified", detail: 'its window shows the folder: "My App - Cursor"' },
        { step: "mcp-connected", label: "MCP connected", status: "not-verified", detail: "not seen within 20 s: make sure tumr is switched on in Cursor Settings → MCP" },
        { step: "session-ready", label: "session ready", status: "not-verified", detail: "TUMR sees this when the agent first uses a TUMR tool (give it a task)" },
      ],
    };
  }
  if (req.action === "plan" || req.action === "apply") {
    if (req.action === "apply") connected[id] = !req.remove;
    return {
      name: id,
      scope: req.scope ?? "project",
      remove: !!req.remove,
      client_exists: !!connected[id],
      applied: req.action === "apply",
      next_step: req.remove ? "Disconnected; restart the agent if it is open." : "Start the agent in your project; it lists the tumr server.",
      changes: [
        {
          path: `${project}\\.cursor\\mcp.json`,
          action: req.remove ? "delete" : "create",
          diff: req.remove ? entryDiff(id).replace(/^\+/gm, "-") : entryDiff(id),
        },
      ],
    };
  }
  return [
    {
      client: id,
      fixed: [],
      checks: [
        { result: "ok", what: `project scope: ${project}\\.cursor\\mcp.json`, fix: null },
        { result: "ok", what: `token for client '${id}'`, fix: null },
        { result: "ok", what: "no tool calls from this agent yet", fix: null },
        { result: "ok", what: "MCP handshake: 16 tools", fix: null },
      ],
    },
  ];
}
