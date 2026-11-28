plugins {
    alias(libs.plugins.android.application)
}

android {
    namespace = "{{PACKAGE}}"
    compileSdk = {{COMPILE_SDK}}

    defaultConfig {
        applicationId = "{{PACKAGE}}"
        minSdk = 24
        targetSdk = {{COMPILE_SDK}}
        versionCode = 1
        versionName = "1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}
