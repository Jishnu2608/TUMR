//! The five framework choices of TUMR Create. Kotlin and Java are separate choices with separate
//! creators; for running, TUMR's detection treats both as one native Android kind.

use serde::{Deserialize, Serialize};

/// A framework the user can create a project for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Framework {
    Expo,
    ReactNative,
    Flutter,
    Kotlin,
    Java,
}

impl Framework {
    /// Every choice, in the order the wizards list them.
    pub const ALL: [Framework; 5] = [
        Framework::Expo,
        Framework::ReactNative,
        Framework::Flutter,
        Framework::Kotlin,
        Framework::Java,
    ];

    /// The id used in requests, flags and metadata (`react-native`).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::Expo => "expo",
            Self::ReactNative => "react-native",
            Self::Flutter => "flutter",
            Self::Kotlin => "kotlin",
            Self::Java => "java",
        }
    }

    #[must_use]
    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|f| f.id() == id.trim().to_ascii_lowercase())
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Expo => "Expo",
            Self::ReactNative => "React Native",
            Self::Flutter => "Flutter",
            Self::Kotlin => "Kotlin",
            Self::Java => "Java",
        }
    }

    /// One line for the wizards.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::Expo => {
                "React Native with Expo, TypeScript; runs in Expo Go without a native build"
            }
            Self::ReactNative => {
                "React Native (Community CLI), TypeScript, with the native Android project"
            }
            Self::Flutter => "Flutter and Dart, with the Android platform project",
            Self::Kotlin => "Native Android in Kotlin with Jetpack Compose",
            Self::Java => "Native Android in Java with XML layouts",
        }
    }

    /// The main language, recorded in the project metadata.
    #[must_use]
    pub fn language(self) -> &'static str {
        match self {
            Self::Expo | Self::ReactNative => "typescript",
            Self::Flutter => "dart",
            Self::Kotlin => "kotlin",
            Self::Java => "java",
        }
    }

    /// What TUMR's detection reports for a project of this framework.
    #[must_use]
    pub fn detected_as(self) -> tumr_resolver::project::Framework {
        use tumr_resolver::project::Framework as D;
        match self {
            Self::Expo => D::Expo,
            Self::ReactNative => D::ReactNative,
            Self::Flutter => D::Flutter,
            Self::Kotlin | Self::Java => D::AndroidNative,
        }
    }

    /// The adapter that runs the project (`tumr run`), as recorded in recent projects.
    #[must_use]
    pub fn adapter(self) -> &'static str {
        match self {
            Self::Expo | Self::ReactNative => "react-native",
            Self::Flutter => "flutter",
            Self::Kotlin | Self::Java => "gradle-android",
        }
    }

    /// Plan components (`tumr plan` ids) the framework's creation tool needs. Building and running
    /// need more (JDK, Android SDK, emulator); `tumr run` plans those as for any project.
    #[must_use]
    pub fn creation_tools(self) -> &'static [&'static str] {
        match self {
            Self::Expo | Self::ReactNative => &["node"],
            Self::Flutter => &["flutter", "git"],
            Self::Kotlin => &["jdk", "android-cmdline-tools"],
            // TUMR's own template: nothing to run at creation
            Self::Java => &[],
        }
    }

    /// Does the framework use an Android application id (`com.example.app`)?
    #[must_use]
    pub fn uses_package_id(self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Framework;

    #[test]
    fn ids_round_trip_and_kotlin_and_java_stay_distinct() {
        for f in Framework::ALL {
            assert_eq!(Framework::parse(f.id()), Some(f));
        }
        assert_eq!(
            Framework::parse(" React-Native "),
            Some(Framework::ReactNative)
        );
        assert_eq!(Framework::parse("android"), None);
        assert_ne!(Framework::Kotlin.id(), Framework::Java.id());
        assert_ne!(Framework::Kotlin.language(), Framework::Java.language());
        // both run through the Gradle adapter
        assert_eq!(Framework::Kotlin.adapter(), Framework::Java.adapter());
    }
}
