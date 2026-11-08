package husk

class Chaff {
    fun sift() = 1
}

fun winnowBefore() = 1
/* Kotlin comments nest: /* this one */ closes only the inner one
class Chaff {
    fun sift() = 0
}
fun winnowBefore() = 0
fun winnowAfter() = 0
val chaffOnly = 0
*/
fun winnowAfter() = 1
/** Reads src/**/*.kt, and the comment ends here. */
fun winnowGlob() = 1

fun thresh(c: Chaff) {
    Chaff()
//  ^ d: src/main/kotlin/husk/Chaff.kt:3
    c.sift()
//    ^ d: src/main/kotlin/husk/Chaff.kt:4
    winnowBefore()
//  ^ d: src/main/kotlin/husk/Chaff.kt:7
    winnowAfter()
//  ^ d: src/main/kotlin/husk/Chaff.kt:16
    println(chaffOnly)
//          ^ d: none
    winnowGlob()
//  ^ d: src/main/kotlin/husk/Chaff.kt:18
}
