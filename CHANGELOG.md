# Changelog

Release notes of Toolchain Universal Mobile Runtime. The **Release build** workflow takes the
section whose heading is the version in `Cargo.toml` (`## 0.1.34`) as the notes of the GitHub
release it drafts, so add that section before you run it. Changes not released yet go under
`## Unreleased` until then.

## Unreleased

## 0.1.36

- **Create a new project (TUMR Create).** Start from nothing: **Create a new project** in the app,
  or `tumr create` in PowerShell, makes an empty Expo, React Native, Flutter, Kotlin or Java
  project with each framework's official tool (Kotlin through Google's Android CLI, Java from
  TUMR's own template with the official Gradle wrapper). You get a minimal starting point, never a
  sample app. TUMR checks the tools the framework needs first and offers to install a missing one,
  and it checks the new project before it trusts it. If anything fails or you cancel, everything
  it made is removed. Projects can go in a new folder or an existing empty one; paths with spaces
  work. Guide: [Create a new project](https://github.com/Jishnu2608/TUMR/blob/main/docs/user/create-project.md).
- **Your agent, connected and opened.** Pick Claude Code, Codex, Cursor, Google Antigravity or
  GitHub Copilot (VS Code) for the new project. TUMR adds the `tumr` server to the agent's own
  settings once, if it is not there yet, plus the project's instructions. It shows every change
  first and backs up each file it changes. It can then open the agent in the project and say
  what it saw: app launched, folder opened, MCP connected, first tool call. It never shows
  something as done that it could not see. New commands: `tumr agent connect` and
  `tumr agent open`; `tumr agent clients` now shows each agent's state (ready, not configured,
  not installed, unsupported).
- **Update toast with your choice of how to update.** When a newer version is out, a small toast
  shows in the app and in the phone window: "Version X is available (you have Y)", a link to
  what's new, and three ways to update: **Update now** (as before: download, checksum check,
  install, reopen), **Update from the terminal** (copies `tumr update` to paste in PowerShell), or
  **Download the installer** (opens it on the release page). **Later** hides it until the next
  version. An open app checks again every 8 hours, not only at start. When the app is minimised
  or behind other windows, a Windows notification says a new version is out (once per version).
  The `tumr` command line tells you too, at most once a day, after a command run in a terminal;
  scripts, agents and `--json` output never see it, and `manifest_url = "off"` turns it off.
- **GPG signature checks for the JDK and Node.js.** On top of the checksum and the Windows
  publisher signature, the Temurin JDK download is checked against Eclipse Adoptium's GPG
  signature, and Node's checksum list against the Node.js release managers' signatures. A
  signature that does not match stops the install. A signature by a key this version does not
  know yet (for example a new Node releaser) installs with a warning.
- **Expo dev builds open straight into your app.** A development build now opens from Metro
  through its development link, at first launch and on restart, instead of stopping at the dev
  launcher. A missing Expo native module becomes one problem naming the package, with the
  `npx expo install` fix. A black emulator screen after a start or reload (a broken GPU session) is
  detected and reported with the fix. Warnings from dependencies go to the logs instead of the
  problems list.
- **Faster native rebuilds.** Gradle runs with its build cache (a clean rebuild of the Expo sample
  went from 96 s to 50 s). A full reload no longer leaves the old Metro running, and stopping
  cancels a reload in progress.
- **Windows path limit recognised.** When a React Native native module's C++ build fails with
  ninja's "Filename longer than 260 characters", the problem is now an environment problem (not
  your code) and comes with the fix: move the project to a shorter folder, delete
  `android\app\.cxx` and run again. The fix shows in the app, VS Code, `tumr problems` and in
  what your agent receives.
- The emulator no longer pops up "Screenshot taken with bug report" a while after it boots (the
  system image's virtual UWB chip filed a bug report; TUMR now switches UWB off).
- **VS Code extension 0.1.7:** problems from the runtime show what to do when the fix is known.
- Copies older than 0.1.34 have no update channel: update them once by hand with the installer
  from the release page.

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
