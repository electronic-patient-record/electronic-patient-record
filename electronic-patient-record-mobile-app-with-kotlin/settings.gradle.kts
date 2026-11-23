// Electronic Patient Record for Android (spec/mobile-apps, MP-2). NOT FOR CLINICAL USE.
// Lily is not published yet, so its Kotlin build is included from the pinned checkout that `sh ../scripts/fetch-lily.sh`
// makes; Gradle substitutes io.github.lilydesignsystem:lily-compose by the included project (MP-3 replaces this).
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}
rootProject.name = "electronic-patient-record-mobile-app-with-kotlin"
includeBuild("../vendor/lily-design-system/lily-design-system-kotlin")
include(":core", ":app")
