package tubs

fun stirOutside(ladle: java.io.File) {
    ladle.stir()
    //    ^ d: !jump
}

fun stirLambda(xs: List<Int>) {
    xs.forEach { ladle -> ladle.stir() }
    //                          ^ d: !jump
}

fun stirOther(spoon: java.io.File) {
    spoon.stir()
    //    ^ d: !jump
}

fun stirPackage() {
    ladle.stir()
    //    ^ d: src/main/kotlin/ladle/Stir.kt:3
}

class Stirs {
    fun mortar(): Int = 1

    fun go() {
        mortar.triturate()
        //     ^ d: src/main/kotlin/mortar/Grind.kt:3
    }
}

class Ladled(val paddle: java.io.File) {
    fun go() {
        paddle.stir()
        //     ^ d: !jump
    }
}

class Wrapped(
    private val paddle: java.io.File,
) {
    fun go() {
        paddle.stir()
        //     ^ d: !jump
    }
}

class Plain(ladle: java.io.File) {
    val stirred = ladle.stir()
    //                  ^ d: !jump

    fun go() {
        ladle.stir()
        //    ^ d: src/main/kotlin/ladle/Stir.kt:3
    }

    fun once() = ladle.stir()
    //                 ^ d: src/main/kotlin/ladle/Stir.kt:3
}

data class Kept(val paddle: java.io.File) {
    fun go() = paddle.stir()
    //                ^ d: !jump
}

class Pot(ladle: java.io.File) {
    init {
        ladle.stir()
        //    ^ d: !jump
    }

    inner class Lid {
        val stirred = ladle.stir()
        //                  ^ d: src/main/kotlin/ladle/Stir.kt:3
    }
}
