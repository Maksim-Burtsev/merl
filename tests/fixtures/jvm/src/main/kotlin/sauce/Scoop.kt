package sauce

// Inside a receiver lambda of Pour, the bare name Saucer is Pour's property, a Ramekin (#1514).
class Scoop {
    fun scoop(pour: Pour): Int = with(pour) { Saucer.make().splash() }
    //                                                      ^ d: src/main/kotlin/sauce/Ramekin.kt:4
}
