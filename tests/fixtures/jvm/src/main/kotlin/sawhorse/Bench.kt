package sawhorse

fun bench() {
    val s = Stool()
    s.hoist().whittle()
//            ^ d: src/main/kotlin/sawhorse/Peg.kt:4
    val e = Easel()
    e.hoist().whittle()
//            ^ d: src/main/kotlin/sawhorse/Peg.kt:4
    Bucksaw().hoist().whittle()
//                    ^ d: src/main/kotlin/sawhorse/Twig.kt:4; want src/main/kotlin/sawhorse/Peg.kt:4 (#1522)
    s.grip(Peg()).whittle()
//                ^ d: picker src/main/kotlin/sawhorse/Peg.kt:4, src/main/kotlin/sawhorse/Twig.kt:4
}
