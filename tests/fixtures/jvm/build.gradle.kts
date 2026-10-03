plugins {
    alias(libs.plugins.compose) apply false
//  ^ d: none
// status: no definition for alias
//                                ^ d: none
//                     ^ d: gradle/libs.versions.toml:13
// status: compose: via gradle/libs.versions.toml
//        ^ d: gradle/libs.versions.toml:13
//             ^ d: gradle/libs.versions.toml:13
}

dependencies {
    implementation(libs.google.oss.licenses)
//                             ^ d: gradle/libs.versions.toml:6
// status: oss: via gradle/libs.versions.toml
//                                 ^ d: gradle/libs.versions.toml:6
    implementation(libs.bundles.compose.ui)
//                                      ^ d: gradle/libs.versions.toml:10
    implementation(libs.androidx.compose.ui)
//                               ^ d: gradle/libs.versions.toml:7
    implementation(libs.versions.googleOss.get())
//                               ^ d: gradle/libs.versions.toml:2
    implementation(libs.missing.thing)
//                      ^ d: none
// status: no definition for missing
}
