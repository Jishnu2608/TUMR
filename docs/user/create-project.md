# Create a new project (TUMR Create)

TUMR Create makes a new, empty mobile project with each framework's official tool, checks that the runtime recognises it, connects your coding agent to it, and can open the agent in it. Your agent then builds the app. TUMR never makes a sample app: you get the framework's minimal starting point.

There are two ways in, and they do the same thing on the same engine (a test checks that both produce identical projects):

- **Desktop app:** **Create a new project** on the home screen, or the link on the setup wizard's **Project** step.
- **Command line:** `tumr create` in PowerShell.

## The frameworks

| Choice | Made with | Needs on your PC | What you get |
|---|---|---|---|
| **Expo** | `create-expo-app` 5.0.0, `blank-typescript` template | Node.js | React Native with Expo in TypeScript, set up for **Expo Go**: it runs without a native Android build. A native build (`expo prebuild`) can be added later. |
| **React Native** | `@react-native-community/cli` 21.0.0, React Native 0.88.1 | Node.js | React Native in TypeScript with its native `android` project. `App.tsx` is a minimal screen showing the app's name instead of the welcome demo. |
| **Flutter** | `flutter create --empty --platforms android` (Flutter 3.50.0) | Flutter SDK and Git | Flutter and Dart with the Android platform project. |
| **Kotlin** | Google's Android CLI, `empty-activity` (Jetpack Compose) | JDK and the Android command-line tools | Native Android in Kotlin. The Android CLI template contains a sample app; TUMR keeps the official project and replaces the sample with one screen that shows the app's name. The Gradle file still lists the libraries the sample used. |
| **Java** | TUMR's own template (the Android CLI has only Kotlin templates) | Nothing | Native Android in Java with an XML layout: one activity, one layout, no libraries, Android Gradle plugin 9.0.1, compile SDK 37. It ships the official Gradle 9.1.0 wrapper; its checksum is checked before every creation. |

TUMR also tidies what the tools leave behind:
- **Expo:** the template's `LICENSE`, `AGENTS.md` and `.claude` folder are removed, and so is the Git repository the tool starts. TUMR never starts a Git repository for you.
- **React Native:** the Android sources are moved to your package's folder.
- **Everywhere:** the app's name and package id are set where each framework keeps them.

Creating a project does not download its packages. The first run does: `tumr run` (or **Run** in the app) runs `npm install` for Expo and React Native, and Flutter fetches its packages itself.

### Prerequisites
TUMR checks the tools above before it writes anything. If one is missing, it offers to install it into its own managed folder, after you read and accept the tool's licence (no global `PATH`, `JAVA_HOME` or `ANDROID_HOME` changes). Building and running the app later needs more (the Android SDK, an emulator image); the environment check lists them, and **Run** offers to install them when you first run the project.

## Name, package id and folder
- **Name:** 1 to 64 characters. It starts with a letter, then letters, digits, spaces, `-` and `_`.
- **Package id:** the Android application id, such as `com.example.myapp`, with at least two parts. Each part starts with a letter and uses letters, digits and `_`. Java keywords are not allowed. It is suggested from the name.
- **Folder:** one of two choices.
  - **A new folder inside a folder you choose.** TUMR creates `<folder>\<name>`, and refuses if it already exists.
  - **An existing, empty folder**, which becomes the project. TUMR refuses a folder with anything in it.

  Paths with spaces work; quotes around a pasted path are removed. TUMR records the folder's full path, so a short Windows name such as `C:\Users\JOHNSM~1` becomes the real one.
- **Long paths:** for Expo and React Native, TUMR warns when the project path is longer than 60 characters. Native C++ builds of their modules can hit Windows' 260-character limit, so a short folder such as `C:\dev` is safer.

TUMR never overwrites anything. If the creation fails or you cancel it, TUMR removes only what this creation made. It also removes the new folder, if it made one.

## Trust
A build runs the project's own code (Gradle scripts, npm scripts, Flutter plugins), so the runtime only builds folders you trust. **Trust the new project** is ticked by default and covers only this folder. TUMR trusts the project only after it has checked it.

If the folder holds files the framework's tool does not normally make, TUMR does not trust it yet. It lists those files and asks you to confirm. Unexpected files are files the tool added or that you put there.

## Your coding agent
Choose one of five agents, or none. Each is shown with its state on your PC:

| State | Meaning |
|---|---|
| **ready** | Installed, and its settings already start the `tumr` server |
| **not configured** | Installed; TUMR can connect it |
| **not installed** | Not found on this PC |
| **unsupported** | Something of it was found, but TUMR has no way to start it |

### Connecting (MCP)
After the project is created, TUMR shows what connecting the agent changes, file by file, and writes it only when you agree:
- **Your settings:** the agent's own settings (all projects) get the `tumr` MCP server **once**, and only if they do not start it already.
- **This project:** gets the agent's instructions file, for example `.cursor/rules/tumr-runtime.mdc`, `.agents/rules/tumr-runtime.md`, or a marked section in `AGENTS.md`.
- **Nothing else changes:** other servers and settings are kept, and the `tumr` server is never listed twice.
- **Backups:** every existing file TUMR changes is copied first to `%USERPROFILE%\.tumr\backups\agent-config\<time>\`, with an `index.txt` that names the originals.

The same step runs on its own as `tumr agent connect <agent> --project <folder>`; it only lists the changes, and `--yes` makes them. The files each agent reads are listed in the [Agents](agents.md) guide.

### Opening the agent
TUMR can start the agent in the new project. It reports each step as it saw it, and never claims more:

| Step | Marked done when TUMR saw |
|---|---|
| MCP config written | A settings file of the agent that starts `tumr` |
| App launched | The agent's program running |
| Workspace opened | A terminal started in the folder, or an editor window whose title names the folder |
| MCP connected | The agent's `tumr` server logging in to the runtime |
| Session ready | The agent's first call to a TUMR tool |

A question mark means TUMR could not see it, or not yet. **MCP connected** often appears only later. For example, Claude Code starts its servers after you accept its folder trust prompt, and VS Code starts them when Chat needs them. **Session ready** appears when you give the agent its first task.

| Agent | How TUMR opens it | Limits (checked on Windows 11) |
|---|---|---|
| Claude Code | A new PowerShell window in the project runs `claude` | MCP connects after you accept the folder trust prompt |
| Codex | A `codex` CLI on `PATH` runs like Claude Code; otherwise the Codex app starts | The Codex app has no supported way to be given a folder: open the project in the app yourself, or right-click the folder in File Explorer and choose its Codex entry |
| Cursor | `cursor <folder>` | Recent Cursor versions open an **Agents** window; TUMR may not see the folder open |
| Google Antigravity | `Antigravity.exe <folder>` | TUMR cannot see which folder the app opened |
| GitHub Copilot | `code <folder>` (VS Code with Copilot agent mode) | VS Code starts `tumr` when Chat in agent mode needs it |

The same step runs on its own as `tumr agent open <agent> [folder]`.

## In the desktop app
**Create a new project** walks through **Framework**, **Name**, **Location**, **Agent**, **Check**, **Create** and **Done**:

1. **Framework** to **Agent:** the fields and choices above. Mistakes show while you type. The folder is checked by the runtime as you choose it, with Windows' folder picker or a typed path.
2. **Check:** lists the tools the framework needs, with **Install** for a missing one. Its licence is shown first.
3. **Create:** shows a summary first. While the project is made, each step appears; **Cancel** stops the tool and removes what was made. If it fails, you see where and the tool's last output, with **Back to the check** and **Try again**.
4. **Done:**
   - **Project:** the folder, how sure TUMR is of the framework, and the package id.
   - **Unexpected files:** if there are any, the files are listed with **Trust it anyway**.
   - **Agent:** the connection's changes, then **Open** with each step marked as above.
   - **Next:** **Run it** starts the app on the emulator. **See what running needs** shows the plan.

## On the command line
Run `tumr create` in a terminal and it asks for what you did not give as an option. Every step has an option, so scripts and agents can create a project without questions:

```powershell
tumr create --framework flutter --name "My App" --parent "D:\Work" --agent cursor --yes
```

| Option | Meaning |
|---|---|
| `--framework` | `expo`, `react-native`, `flutter`, `kotlin` or `java` |
| `--name` | The project's name (also the new folder's name with `--parent`) |
| `--package-id` | Android application id; default `com.example.<name>` |
| `--parent <folder>` | Create a new folder named after the project in this folder (default: the current folder) |
| `--path <folder>` | An existing, empty folder that becomes the project (instead of `--parent`) |
| `--agent` | `claude-code`, `codex`, `cursor`, `antigravity`, `copilot` or `none` |
| `--no-trust` | Do not trust the new folder |
| `--install` | Install a missing creation tool without asking first (its licence is still shown, and you still accept it) |
| `--open` / `--no-open` | Open the agent afterwards, or not |
| `--yes` | Never ask: values with a default take it, and confirmations are given. Trust despite unexpected files still needs a person. |
| `--json` | A machine-readable result; never asks |

**Ctrl+C** stops at any time. Before the summary nothing has been written. During the creation, TUMR stops the tool and removes what it made.

| Exit code | Meaning |
|---|---|
| 0 | Created |
| 1 | The creation failed (and was rolled back), or the runtime failed |
| 2 | Invalid input, or a value is missing and there is no terminal to ask in |
| 3 | Cancelled (Ctrl+C, end of input, or "no" at the summary) |
| 4 | A tool the framework's creator needs is not installed (nothing was created) |

## Troubleshooting

| You see | What to do |
|---|---|
| "… is not empty; choose an empty folder or a parent folder for a new one" | Pick an empty folder, or choose **A new folder** and pick the folder it should go in |
| "… already exists; TUMR never overwrites an existing folder" | Choose another name or another parent folder |
| "Flutter needs Flutter SDK first" (exit code 4) | Install it from the environment check (**Install**), or run `tumr create … --install` in a terminal |
| "create-expo-app failed (exit code 1). Last output: … network …" | The official tools download their templates: check your internet connection or proxy, then **Try again**. Nothing was left behind |
| "the project path is … characters long" | A warning for Expo and React Native: move the project to a short folder such as `C:\dev` before the first native build |
| "Not trusted yet … files TUMR did not expect" | Look at the listed files. If you know them, **Trust it anyway** (or `tumr trust <folder>`) |
| "the Gradle wrapper shipped with TUMR is damaged" | Reinstall TUMR; the Java template's wrapper did not match its checksum |
| The agent opened but **MCP connected** shows a question mark | Do what the step says for your agent (accept Claude Code's trust prompt, switch `tumr` on in Cursor's MCP settings, open Chat in VS Code), then check with `tumr agent doctor <agent>` |
| The Codex app opened but not your project | Expected: open the folder in the app yourself |

Problems while building or running the new project are covered in [Troubleshooting](troubleshooting.md).

## Limits
- Android only, on Windows 11 x64, like the rest of the runtime.
- The template versions above are the ones TUMR was tested with. The official tools may change their templates in later releases; TUMR's check after each creation reports files it did not expect.
- A full build is not part of creation. The first **Run** builds the app, which takes several minutes for native projects.
- The Java template's Android Gradle plugin and compile SDK versions are fixed in this release.
- TUMR can start the Codex app but not open a folder in it. Whether Cursor and Antigravity opened the folder is not always visible to TUMR.
