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
    //                         ^ d: picker src/main/kotlin/cauldron/use/Called.kt:18, src/main/kotlin/cauldron/lustre/Burnish.kt:5, src/main/kotlin/cauldron/matte/Burnish.kt:5; want src/main/kotlin/cauldron/lustre/Burnish.kt:5 (#1499)
}
