package ladle

fun serve(): Int {
    return Ladle.dollop() + Ladle.brim
//               ^ d: src/main/kotlin/ladle/LadleExt.kt:3
//                                ^ d: src/main/kotlin/ladle/LadleExt.kt:5
}
