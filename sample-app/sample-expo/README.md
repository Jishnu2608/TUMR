# Calculator — Expo dev build (TypeScript)

The same calculator as `sample-react-native`, as an **Expo** app on the **dev-build path**: the
`android` folder (made once with `npx expo prebuild --platform android`) holds the native project,
and `expo-dev-client` is installed. The Toolchain Universal Mobile Runtime builds that native app
with Gradle, installs it on the emulator and starts Metro with `expo start --dev-client`; your
TypeScript reloads with Fast Refresh.

(An Expo app **without** an `android` folder runs in Expo Go instead, with no native build.)

What it does: digits, `.`, `+ − × ÷`, `=`, `AC`, `⌫`, `±` and `%`, with the pending calculation
shown above the result. Dividing by zero shows `Error`.

## Run it

```powershell
tumr trust .
tumr run
```

Or in VS Code with the **Toolchain Universal Mobile Runtime** extension: open this folder and run
**Run on Emulator**. Build errors and crashes appear in the **Problems** panel with file and line.

The calculator rules live in `src/calculator.ts` (shared with the React Native sample); the screen
is `App.tsx`. The first run installs the JavaScript dependencies and builds the native app, which
takes several minutes once.

## Keep the path short

Native builds on Windows hit the 260-character path limit in deep folders. If the build fails
with "Filename longer than 260 characters", copy this folder to a short path such as
`C:\dev\calc-expo` and run it from there.
