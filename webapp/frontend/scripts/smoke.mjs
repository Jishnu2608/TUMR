// R0 safety net for the redesign (frontend_enhancements.md): checks the built static site in
// `out/` for the functionality every phase must keep. Run after `npm run build`.
// Exit code 1 lists what is missing.
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const out = join(process.cwd(), "out");
const fail = [];
const read = (p) => readFileSync(join(out, p), "utf8");
const check = (ok, what) => {
  if (!ok) fail.push(what);
};

// routes
const routes = [
  "index.html",
  "download/index.html",
  "releases/index.html",
  "docs/index.html",
  "docs/getting-started/index.html",
  "docs/reference/mcp/index.html",
  "docs/guides/vscode/index.html",
  "security/index.html",
  "privacy/index.html",
  "licences/index.html",
  "open-source/index.html",
  "404.html",
  "search-index.json",
];
for (const r of routes) check(existsSync(join(out, r)), `route missing: ${r}`);
if (fail.length) report();

const home = read("index.html");

// theme script runs before paint, theme toggle and skip link exist
check(/localStorage\.getItem\("theme"\)/.test(home), "no-flash theme script missing");
check(/Switch to (dark|light) theme|aria-label="[^"]*theme/i.test(home), "theme toggle missing");
check(/href="#main"/.test(home), "skip link missing");

// navigation and footer links
for (const href of ["/docs/getting-started/", "/download/", "/releases/", "/security/", "/privacy/", "/licences/", "/open-source/"]) {
  check(home.includes(`href="${href}"`), `link missing on the homepage: ${href}`);
}

// homepage sections (headings referenced by aria-labelledby)
for (const id of ["hero-title", "pillars-title", "how-title", "frameworks-title", "agents-title", "vscode-title", "arch-title", "privacy-title", "oss-title", "faq-title", "close-title"]) {
  check(home.includes(`id="${id}"`), `homepage section missing: ${id}`);
}

// MCP catalogue: every tool from content/mcp-tools.json is listed on the homepage
const tools = JSON.parse(readFileSync(join(process.cwd(), "content", "mcp-tools.json"), "utf8"));
for (const t of tools) check(home.includes(t.name), `MCP tool missing on the homepage: ${t.name}`);

// download logic: the latest stable installer from the manifest is linked with its checksum
const manifest = JSON.parse(readFileSync(join(process.cwd(), "content", "releases.json"), "utf8").replace(/^﻿/, ""));
const stable = manifest.releases.filter((r) => r.channel === "stable");
if (stable.length) {
  const latest = stable.sort((a, b) => b.version.localeCompare(a.version, undefined, { numeric: true }))[0];
  const installer = latest.files.find((f) => f.kind === "installer");
  const dl = read("download/index.html");
  check(dl.includes(installer.url), `download page lacks the ${latest.version} installer link`);
  check(dl.includes(installer.sha256), `download page lacks the ${latest.version} installer SHA-256`);
  check(home.includes("Download for Windows"), "homepage CTA does not offer the download");
}

// VS Code store links
const site = readFileSync(join(process.cwd(), "lib", "site.ts"), "utf8");
for (const store of ["marketplace", "openVsx"]) {
  const m = site.match(new RegExp(`${store}: \\{\\s*url: "([^"]+)",\\s*published: (true|false)`));
  if (m && m[2] === "true") check(read("download/index.html").includes(m[1]), `download page lacks the ${store} link`);
}

// search index
try {
  const idx = JSON.parse(read("search-index.json"));
  check(Array.isArray(idx) ? idx.length > 10 : Object.keys(idx).length > 0, "search index is empty");
} catch {
  fail.push("search-index.json is not valid JSON");
}

// documentation pages render their MDX (a heading and body text)
check(/<h1[^>]*>[^<]+/.test(read("docs/getting-started/index.html")), "getting-started page has no heading");

report();

function report() {
  if (fail.length) {
    console.error(`smoke: ${fail.length} problem(s)`);
    for (const f of fail) console.error(`  - ${f}`);
    process.exit(1);
  }
  console.log(`smoke: ${routes.length} routes, navigation, sections, ${tools.length} MCP tools, download, stores, search, docs — all present`);
}
