package imports.network

data class Badge(val id: String, val url: String)

fun feed(badge: Badge) = badge
//              ^ d: src/main/kotlin/imports/network/Badge.kt:3
