package cauldron.use.called

import cauldron.Cauldron
import cauldron.lustre.burnish

fun pot2(): cauldron.Cauldron = Cauldron()

fun pot3(): Cauldron = Cauldron()

fun burnishCalled() {
    pot2().burnish()
    //     ^ d: picker src/main/kotlin/cauldron/lustre/Burnish.kt:5, src/main/kotlin/cauldron/matte/Burnish.kt:5; want src/main/kotlin/cauldron/lustre/Burnish.kt:5 (#1499)
    pot3().burnish()
    //     ^ d: picker src/main/kotlin/cauldron/lustre/Burnish.kt:5, src/main/kotlin/cauldron/matte/Burnish.kt:5; want src/main/kotlin/cauldron/lustre/Burnish.kt:5 (#1499)
}
