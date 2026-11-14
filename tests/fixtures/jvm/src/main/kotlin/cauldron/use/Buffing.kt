package cauldron.use

import cauldron.Cauldron
import cauldron.buff.BuffA.buff

fun buffOne(pot: Cauldron) {
    pot.buff()
    //  ^ d: picker src/main/kotlin/cauldron/buff/BuffA.kt:6, src/main/kotlin/cauldron/use/Buff.kt:5
}
