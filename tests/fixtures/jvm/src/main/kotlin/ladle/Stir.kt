package ladle

fun stir(): Int {
    return Spoon.swirl() + Spoon.scoop()
//               ^ d: src/main/kotlin/ladle/LadleExt.kt:8
//                               ^ d: src/main/kotlin/ladle/LadleExt.kt:10
}
