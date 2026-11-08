package gambit

import gambit.Caper.*

fun board(piece: Caper): Int = when (piece) {
    LARK -> 1
 // ^ d: none; want src/main/kotlin/gambit/Caper.kt:4 (#1448)
    JEST -> 0
}
