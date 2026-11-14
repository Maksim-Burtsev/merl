package ladle

fun garnish(): Int = 0

class Sprig {
    fun garnish(): Int = 5

    fun plate(): Int = garnish()
    //                 ^ d: src/main/kotlin/ladle/Garnish.kt:6
}

fun plateTop(): Int = garnish()
//                    ^ d: src/main/kotlin/ladle/Garnish.kt:3

fun plateWith(s: Sprig): Int = with(s) { garnish() }
//                                       ^ d: src/main/kotlin/ladle/Garnish.kt:6

class Bowl {
    fun smudge(): Int = 2

    fun smear(): Int = 2
}

fun Bowl.smudge(): Int = 1

fun plateSmudge(): Int = smudge()
//                       ^ d: picker src/main/kotlin/ladle/Garnish.kt:19, src/main/kotlin/ladle/Garnish.kt:24

fun plateSmear(): Int = smear()
//                      ^ d: picker src/main/kotlin/ladle/Garnish.kt:21, src/main/kotlin/smears/Smear.kt:3

fun Sprig.plateExt(): Int = garnish()
//                          ^ d: src/main/kotlin/ladle/Garnish.kt:6

context(Sprig)
fun plateCtx(): Int = garnish()
//                    ^ d: !jump

fun plateSprinkle() = sprinkle()
//                    ^ d: src/main/kotlin/ladle/Spoon.kt:7

fun plateSprinkleBlock(): Int {
    return sprinkle()
    //     ^ d: src/main/kotlin/ladle/Spoon.kt:7
}

class Shaker {
    fun shake(): Int = sprinkle()
    //                 ^ d: src/main/kotlin/ladle/Spoon.kt:7
}
