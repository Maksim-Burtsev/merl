package quaich

typealias Quaich = quaich.cups.Tankard

fun brew(): Int {
    val k = Quaich()
    //      ^ d: src/main/kotlin/quaich/Brew.kt:3
    k.swig()
    //^ d: src/main/kotlin/quaich/cups/Cups.kt:4
    return Quaich().swig()
    //              ^ d: src/main/kotlin/quaich/cups/Cups.kt:4
}
