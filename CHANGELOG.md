# Changelog

Release notes of Toolchain Universal Mobile Runtime. The **Release build** workflow takes the
section whose heading is the version in `Cargo.toml` (`## 0.1.34`) as the notes of the GitHub
release it drafts, so add that section before you run it. Changes not released yet go under
`## Unreleased` until then.

## Unreleased

- The phone window docks to the right of the screen when an agent starts the app, the same as
  `tumr run`; in VS Code the Mobile Preview opens to the right of the code.
- Update check on by default: at start the app reads the release list of the project's GitHub
  Releases and offers **Update now**. Set `manifest_url = "off"` under `[update]` in
  `%USERPROFILE%\.tumr\config.toml` to turn it off.
- Renaming emulators made before the rename retries when Windows briefly holds the folder.

## 0.1.33

First public release (preview, Windows 11 x64).

- Installer (per user, no administrator rights), portable ZIP and command-line ZIP.
- MCP bundle (`.mcpb`) for Claude Desktop and the official MCP Registry.
- VS Code extension (`.vsix`), also on the VS Code Marketplace and Open VSX.
- Agent plugins for Claude Code, Codex, Cursor and Antigravity; the problem loop sends build
  errors, crashes and runtime errors back to the agent until the app is clean.
- Licensed under Apache-2.0.
