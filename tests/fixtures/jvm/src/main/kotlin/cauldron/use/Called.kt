package cauldron.use.called

import cauldron.Cauldron
import cauldron.lustre.burnish

fun pot2(): cauldron.Cauldron = Cauldron()

fun pot3(): Cauldron = Cauldron()

fun burnishCalled() {
    pot2().burnish()
    //     ^ d: src/main/kotlin/cauldron/lustre/Burnish.kt:5
    pot3().burnish()
    //     ^ d: src/main/kotlin/cauldron/lustre/Burnish.kt:5
}

class Kettle {
    fun burnish() {}
}

fun pot4(): Cauldron = Cauldron()

fun shadowedPot() {
    fun pot4(): Kettle = Kettle()
    pot4().burnish()
    //     ^ d: picker src/main/kotlin/cauldron/use/Called.kt:18, src/main/kotlin/cauldron/lustre/Burnish.kt:5, src/main/kotlin/cauldron/matte/Burnish.kt:5
}

fun bracedPot() {
    listOf(1).forEach { pot3().burnish() }
    //                         ^ d: src/main/kotlin/cauldron/lustre/Burnish.kt:5
}

fun namedPot() {
    listOf(1).map { n ->
        pot3().burnish()
        //     ^ d: src/main/kotlin/cauldron/lustre/Burnish.kt:5
    }
}

class Firedog {
    fun pot3(): Kettle = Kettle()
}

fun receiverPots() {
    Firedog().apply { pot3().burnish() }
    //                       ^ d: picker src/main/kotlin/cauldron/use/Called.kt:18, src/main/kotlin/cauldron/lustre/Burnish.kt:5, src/main/kotlin/cauldron/matte/Burnish.kt:5
    Firedog().run { pot3().burnish() }
    //                     ^ d: picker src/main/kotlin/cauldron/use/Called.kt:18, src/main/kotlin/cauldron/lustre/Burnish.kt:5, src/main/kotlin/cauldron/matte/Burnish.kt:5
    with(Firedog()) { pot3().burnish() }
    //                       ^ d: picker src/main/kotlin/cauldron/use/Called.kt:18, src/main/kotlin/cauldron/lustre/Burnish.kt:5, src/main/kotlin/cauldron/matte/Burnish.kt:5
    buildList<Int> { pot3().burnish() }
    //                      ^ d: picker src/main/kotlin/cauldron/use/Called.kt:18, src/main/kotlin/cauldron/lustre/Burnish.kt:5, src/main/kotlin/cauldron/matte/Burnish.kt:5
}

fun pots(): Pair<Cauldron, Kettle> = Pair(Cauldron(), Kettle())

fun keptPots() {
    listOf(1).forEach {
        val k = pot3()
        k.burnish()
        //^ d: src/main/kotlin/cauldron/lustre/Burnish.kt:5
        val (c, t) = pots()
        c.burnish()
        //^ d: src/main/kotlin/cauldron/lustre/Burnish.kt:5
    }
}

fun localPots() {
    fun Firedog.inner() = listOf(1).forEach {
        pot3().burnish()
        //     ^ d: picker src/main/kotlin/cauldron/use/Called.kt:18, src/main/kotlin/cauldron/lustre/Burnish.kt:5, src/main/kotlin/cauldron/matte/Burnish.kt:5
    }
    with(Firedog().also { })
    {
        pot3().burnish()
        //     ^ d: picker src/main/kotlin/cauldron/use/Called.kt:18, src/main/kotlin/cauldron/lustre/Burnish.kt:5, src/main/kotlin/cauldron/matte/Burnish.kt:5
    }
}
