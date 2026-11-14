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
