# Changelog

Release notes of Toolchain Universal Mobile Runtime. The **Release build** workflow takes the
section whose heading is the version in `Cargo.toml` (`## 0.1.34`) as the notes of the GitHub
release it drafts, so add that section before you run it. Changes not released yet go under
`## Unreleased` until then.

## Unreleased

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
