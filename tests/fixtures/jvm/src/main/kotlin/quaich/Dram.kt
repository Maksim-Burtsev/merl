package quaich

typealias Dram = Quaich

fun dram(): Int = Dram().swig()
//                       ^ d: picker src/main/kotlin/quaich/Nest.kt:5, src/main/kotlin/quaich/cups/Cups.kt:4, src/main/kotlin/quaich/decoy/Quaich.kt:4, src/main/kotlin/quaich/other/Tankard.kt:4; want src/main/kotlin/quaich/cups/Cups.kt:4 (#1466)
