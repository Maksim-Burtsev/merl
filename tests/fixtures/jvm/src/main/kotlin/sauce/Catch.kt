package sauce

// runCatching takes a receiver lambda too: Saucer is Pour's property, a Ramekin (#1514).
class Catch {
    fun caught(pour: Pour): Int = pour.runCatching { Saucer.make().splash() }.getOrDefault(0)
    //                                                             ^ d: src/main/kotlin/sauce/Ramekin.kt:4
}
