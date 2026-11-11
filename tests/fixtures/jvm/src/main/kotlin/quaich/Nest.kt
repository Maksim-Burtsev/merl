package quaich

class Snug {
    class Quaich {
        fun swig() = 4
    }

    fun nurse(q: Quaich) = q.swig()
    //                       ^ d: picker src/main/kotlin/quaich/Nest.kt:5, src/main/kotlin/quaich/cups/Cups.kt:4, src/main/kotlin/quaich/decoy/Quaich.kt:4, src/main/kotlin/quaich/other/Tankard.kt:4; want src/main/kotlin/quaich/Nest.kt:5 (#1466)
}
