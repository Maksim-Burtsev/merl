package sauce

// A private property is not seen from an extension of its class: there Rundlet stays the class,
// which Hutch's property spoils, so d keeps the picker and never jumps to the Cask (#1515).
class Puncheon {
    private val Rundlet = Cask()
}

fun Puncheon.drawn(): Int = Rundlet.make().seethe()
//                                         ^ d: picker src/main/kotlin/sauce/Cask.kt:4, src/main/kotlin/sauce/Kegs.kt:4, src/main/kotlin/sauce/Kegs.kt:12, src/main/kotlin/sauce/Kegs.kt:20
