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
