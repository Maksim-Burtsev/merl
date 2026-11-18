package sauce

// Inside a receiver lambda of Pour, the bare name Saucer is Pour's property, a Ramekin (#1514).
class Scoop {
    fun scoop(pour: Pour): Int = with(pour) { Saucer.make().splash() }
    //                                                      ^ d: picker src/main/kotlin/sauce/Ramekin.kt:4, src/main/kotlin/sauce/Saucer.kt:4; want src/main/kotlin/sauce/Ramekin.kt:4 (#1515)
}
