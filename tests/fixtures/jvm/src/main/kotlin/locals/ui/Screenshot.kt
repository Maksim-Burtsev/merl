package locals.ui

fun extractSpecs(specs: Map<String, String>): Int {
    val height = specs["height"]?.toInt() ?: 480
    val followedTopicId = "x"
    return height + followedTopicId.length
    //     ^ d: src/main/kotlin/locals/ui/Screenshot.kt:4
}
