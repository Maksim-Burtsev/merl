package ladle

fun Ladle.Companion.dollop(): Int = SIZE

val Ladle.Companion.brim: Int
    get() = SIZE + 1

fun Spoon.swirl(): Int = DEPTH

fun ladle.Spoon.scoop(): Int = DEPTH
