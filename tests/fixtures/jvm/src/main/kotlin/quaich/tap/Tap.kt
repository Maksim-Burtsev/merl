package quaich.tap

import quaich.Quaich
import quaich.Quaich as Cog

fun nip(): Int {
    val q = Quaich()
    //      ^ d: src/main/kotlin/quaich/Brew.kt:3
    q.swig()
    //^ d: src/main/kotlin/quaich/cups/Cups.kt:4
    Cog().swig()
    //    ^ d: src/main/kotlin/quaich/cups/Cups.kt:4
    return Quaich().swig()
    //              ^ d: src/main/kotlin/quaich/cups/Cups.kt:4
}
