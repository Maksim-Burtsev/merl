package sauce

typealias Horn = Bombard

// Beaker inherits Bombard's property Rhyton, a Situla, through the alias Horn (#1514).
class Beaker : Horn() {
    fun dregs(): Int = Rhyton.make().gurgle()
    //                               ^ d: picker src/main/kotlin/sauce/Rhyton.kt:4, src/main/kotlin/sauce/Rhyton.kt:12; want src/main/kotlin/sauce/Rhyton.kt:12 (#1515)
}
