package sauce

import sauce.Hutch as Jug

// Jug is Hutch imported under another name, so Rundlet is Pantler's inherited property, a Cask
// (#1514).
class Pantler : Jug() {
    fun f(): Int = Rundlet.make().seethe()
    //                            ^ d: src/main/kotlin/sauce/Cask.kt:4
}
