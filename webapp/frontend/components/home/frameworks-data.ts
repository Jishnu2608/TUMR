/**
 * R7: what the runtime does for each framework. Every line comes from the framework guides
 * (content/docs/guide-*.mdx), the requirements page and measured results; nothing here is a
 * promise the adapters do not keep. Kept neutral: each fact is plain text, never a good/bad colour.
 */
export type Framework = {
  id: string;
  name: string;
  language: string;
  /** what builds and serves it, shown under the tab name */
  toolchain: string;
  detect: string;
  installs: string;
  runs: string;
  save: string;
  saveFigure?: string;
  problems: string;
  limits: string;
  guide: { href: string; label: string };
};

const gradle = {
  installs: "The JDK and Android SDK packages that the Gradle wrapper, the Android Gradle Plugin and `compileSdk` ask for.",
  runs: "Gradle builds a debug APK through the project's own wrapper; the runtime installs it and launches the launcher activity.",
  save: "An incremental Gradle build and a reinstall start by themselves. Kotlin and Java have no hot reload outside Android Studio.",
  saveFigure: "About 15 to 20 s once the Gradle daemon is warm",
  limits: "The app restarts on every change, since it is reinstalled. The runtime never edits the project.",
  guide: { href: "/docs/guides/kotlin-java/", label: "Kotlin and Java guide" },
};

export const frameworks: Framework[] = [
  {
    id: "kotlin",
    name: "Kotlin",
    language: "Kotlin",
    toolchain: "Gradle",
    detect: "`build.gradle(.kts)` with an Android application",
    ...gradle,
    problems: "kotlinc and Android resource (AAPT) errors at file and line; Gradle failures without a location with Gradle's own summary.",
  },
  {
    id: "java",
    name: "Java",
    language: "Java",
    toolchain: "Gradle",
    detect: "`build.gradle(.kts)` with an Android application",
    ...gradle,
    problems: "javac and Android resource (AAPT) errors at file and line; Gradle failures without a location with Gradle's own summary.",
  },
  {
    id: "flutter",
    name: "Flutter",
    language: "Dart",
    toolchain: "Flutter tool",
    detect: "`pubspec.yaml` with an `android/` folder",
    installs: "The Flutter SDK that matches the `pubspec.yaml` constraint, Git if missing (MinGit), and the Android parts.",
    runs: "`flutter run` in machine mode, so hot reload, hot restart and the app's log come from the Flutter tool itself.",
    save: "Saved Dart files hot-reload by themselves; `tumr reload restart` resets the state.",
    saveFigure: "Hot reload 0.7 to 1.8 s, measured",
    problems: "Dart compile errors at file and line; Flutter framework errors and unhandled exceptions once each, at the widget or your first frame.",
    limits: "Native changes (Gradle files, plugins, `android/`) need a full reload: `tumr reload full`.",
    guide: { href: "/docs/guides/flutter/", label: "Flutter guide" },
  },
  {
    id: "react-native",
    name: "React Native",
    language: "JavaScript or TypeScript",
    toolchain: "Gradle and Metro",
    detect: "`package.json` with `react-native`",
    installs: "Node.js that matches `engines.node` in `package.json`, and the Android parts.",
    runs: "A native build through Gradle, then Metro on localhost, forwarded to the emulator with `adb reverse`.",
    save: "Fast Refresh applies saved JavaScript and TypeScript by itself.",
    problems: "Babel and Metro syntax errors at file and line; JavaScript runtime errors that Metro reports.",
    limits: "Native changes need `tumr reload full`. Some native modules fail on long project paths, so the runtime refuses those paths up front.",
    guide: { href: "/docs/guides/react-native-expo/", label: "React Native and Expo guide" },
  },
  {
    id: "expo",
    name: "Expo",
    language: "JavaScript or TypeScript",
    toolchain: "Expo Go or a development build",
    detect: "`package.json` with `expo`",
    installs: "Node.js that matches `engines.node` in `package.json`, and the Android parts.",
    runs: "Without an `android/` folder: Expo Go on the emulator, no native build. With one (a development build): a native build for the device, then Metro.",
    save: "Fast Refresh applies saved JavaScript and TypeScript by itself.",
    problems: "Babel and Metro syntax errors at file and line; JavaScript runtime errors that Metro reports.",
    limits: "A development build needs `tumr reload full` after native changes; long project paths are refused, as for React Native.",
    guide: { href: "/docs/guides/react-native-expo/", label: "React Native and Expo guide" },
  },
];

/** The rows of each panel, in reading order. */
export const facts: { key: keyof Framework; label: string }[] = [
  { key: "language", label: "Language" },
  { key: "detect", label: "Detected by" },
  { key: "installs", label: "Installs" },
  { key: "runs", label: "Builds and runs" },
  { key: "save", label: "After you save" },
  { key: "problems", label: "Problems it reports" },
  { key: "limits", label: "Limits" },
];
