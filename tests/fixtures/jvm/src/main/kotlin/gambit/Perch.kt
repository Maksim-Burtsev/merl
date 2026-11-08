package gambit

import gambit.Warbler.LARK

// A case two enums of the project share, written without its enum (#1448).
fun perch(bird: Warbler): Int = when (bird) {
    LARK -> 1
 // ^ d: none; want src/main/kotlin/gambit/Warbler.kt:5 (#1448)
    Warbler.WREN -> 0
}

fun advance(piece: Caper): Int = when (piece) {
    Caper.LARK -> 1
       // ^ d: src/main/kotlin/gambit/Caper.kt:4
    Caper.JEST -> 0
}

fun plain(): Warbler = LARK
                    // ^ d: none; want src/main/kotlin/gambit/Warbler.kt:5 (#1448)
