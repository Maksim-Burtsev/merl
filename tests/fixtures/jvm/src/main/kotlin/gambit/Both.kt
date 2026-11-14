package gambit

import gambit.Caper.*
import gambit.Warbler.LARK

// An explicit import of a case wins over a star import of its namesake's enum (#1448).
fun both(): Warbler = LARK
                   // ^ d: src/main/kotlin/gambit/Warbler.kt:5

fun star(): Caper = JEST
                 // ^ d: src/main/kotlin/gambit/Caper.kt:5
