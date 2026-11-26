# Changelog

Release notes of Toolchain Universal Mobile Runtime. The **Release build** workflow takes the
section whose heading is the version in `Cargo.toml` (`## 0.1.34`) as the notes of the GitHub
release it drafts, so add that section before you run it. Changes not released yet go under
`## Unreleased` until then.

## Unreleased

- **GPG signature checks for the JDK and Node.js.** On top of the checksum and the Windows
  publisher signature, the Temurin JDK download is checked against Eclipse Adoptium's GPG
  signature, and Node's checksum list against the Node.js release managers' signatures. A
  signature that does not match stops the install; a signature by a key this version does not
  know yet (for example a new Node releaser) installs with a warning.

- **Update toast with your choice of how to update.** When a newer version is out, a small toast
  shows in the app and in the phone window: "Version X is available (you have Y)", a link to
  what's new, and three ways to update: **Update now** (as before: download, checksum check,
  install, reopen), **Update from the terminal** (copies `tumr update` to paste in PowerShell), or
  **Download the installer** (opens it on the release page). **Later** hides it until the next
  version. An open app checks again every 8 hours, not only at start.
- When the app is minimised or behind other windows, a Windows notification says a new version is
  out (once per version).
- The `tumr` command line tells you too: at most once a day, after a command run in a terminal, it
  prints "Update available: X → Y. Run `tumr update` to install it." Scripts, agents and piped or
  `--json` output never see it; `manifest_url = "off"` turns it off.
- Copies older than 0.1.34 have no update channel and cannot show this: update them once by hand
  with the installer from the release page.

## 0.1.35

- **New look: Studio / Signal.** The app icon, installer and Start menu icon, the desktop app,
  the VS Code extension and the agent plugins use the new colours: warm porcelain and ink, Signal
  Blue for actions and focus, coral for build progress and warnings, sage for a healthy connection.
  The logo keeps its shape (a phone showing an agent's prompt) on an ink tile with a blue prompt.
- **VS Code extension 0.1.6:** the new icon and the new colours in the Mobile Preview.
- The MCP bundle and the agent plugins carry the new icon.

## 0.1.34

- **Updates find you.** The app now checks the project's GitHub Releases when it starts and
  offers **Update now**; nothing is installed without your click, and only the small release list
  is downloaded. Turn it off with `manifest_url = "off"` under `[update]` in
  `%USERPROFILE%\.tumr\config.toml`. From this version on, new releases reach you by themselves.
- **Phone on the right.** When a coding agent starts the app, the phone window docks to the right
  of the screen, the same as `tumr run`.
- **VS Code extension 0.1.5:** offers **Download the runtime** when the runtime is not installed,
  and the Mobile Preview opens to the right of the code. Now on the VS Code Marketplace and Open VSX.
- Emulators made before the rename to `tumr` are renamed reliably even when Windows briefly holds
  their folder.
- The MCP bundle and the agent plugins link the latest release.

## 0.1.33

First public release (preview, Windows 11 x64).

- Installer (per user, no administrator rights), portable ZIP and command-line ZIP.
- MCP bundle (`.mcpb`) for Claude Desktop and the official MCP Registry.
- VS Code extension (`.vsix`), also on the VS Code Marketplace and Open VSX.
- Agent plugins for Claude Code, Codex, Cursor and Antigravity; the problem loop sends build
  errors, crashes and runtime errors back to the agent until the app is clean.
- Licensed under Apache-2.0.
