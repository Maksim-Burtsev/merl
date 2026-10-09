package pins

class Stack(val n: Int) {
    infix fun onto(other: Stack) = Stack(n + other.n)

    fun onto(x: Int, y: Int) = this
}

val top = Stack(0)

fun pile(a: Stack, b: Stack, flag: Boolean) {
    val c = a onto b
    //        ^ d: src/main/kotlin/pins/Stack.kt:4
    val d = if (flag) a else top as Stack
    //                       ^ d: src/main/kotlin/pins/Stack.kt:9
    while (flag) top onto a
    //           ^ d: src/main/kotlin/pins/Stack.kt:9
}
