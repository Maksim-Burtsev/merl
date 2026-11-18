package sauce

// runCatching takes a receiver lambda too: Saucer is Pour's property, a Ramekin (#1514).
class Catch {
    fun caught(pour: Pour): Int = pour.runCatching { Saucer.make().splash() }.getOrDefault(0)
    //                                                             ^ d: picker src/main/kotlin/sauce/Ramekin.kt:4, src/main/kotlin/sauce/Saucer.kt:4; want src/main/kotlin/sauce/Ramekin.kt:4 (#1515)
}
