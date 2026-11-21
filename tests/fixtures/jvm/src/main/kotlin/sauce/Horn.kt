package sauce

typealias Horn = Bombard

// Beaker inherits Bombard's property Rhyton, a Situla, through the alias Horn (#1514).
class Beaker : Horn() {
    fun dregs(): Int = Rhyton.make().gurgle()
    //                               ^ d: src/main/kotlin/sauce/Rhyton.kt:12

    // A lambda of a library function may have a receiver of its own with a Rhyton (#1515).
    fun brewed(): Int = configure { Rhyton.make().gurgle() }
    //                                            ^ d: picker src/main/kotlin/sauce/Rhyton.kt:4, src/main/kotlin/sauce/Rhyton.kt:12

    // A companion object does not see the instance property Rhyton of Beaker (#1515).
    companion object {
        fun made(): Int = Rhyton.make().gurgle()
        //                              ^ d: picker src/main/kotlin/sauce/Rhyton.kt:4, src/main/kotlin/sauce/Rhyton.kt:12
    }
}
