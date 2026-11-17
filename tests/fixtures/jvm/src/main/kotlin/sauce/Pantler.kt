package sauce

import sauce.Hutch as Jug

// Pantler extends Hutch under the name Jug and inherits its property Rundlet, a Cask (#1514).
class Pantler : Jug() {
    fun f(): Int = Rundlet.make().seethe()
    //                            ^ d: picker src/main/kotlin/sauce/Cask.kt:4, src/main/kotlin/sauce/Kegs.kt:4, src/main/kotlin/sauce/Kegs.kt:12, src/main/kotlin/sauce/Kegs.kt:20; want src/main/kotlin/sauce/Cask.kt:4 (#1515)
}
