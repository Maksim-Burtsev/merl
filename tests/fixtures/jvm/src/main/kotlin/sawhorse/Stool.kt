package sawhorse

open class Stool : Sawhorse<Peg>(Peg()) {
    fun tidy() {
        hoist().whittle()
//              ^ d: src/main/kotlin/sawhorse/Peg.kt:4
    }

    fun sawLift() =
        hoist().whittle()
//              ^ d: src/main/kotlin/sawhorse/Peg.kt:4

    fun sawScoped() {
        Twig().apply {
            hoist().whittle()
//                  ^ d: picker src/main/kotlin/sawhorse/Peg.kt:4, src/main/kotlin/sawhorse/Twig.kt:4
        }
        with(Twig()) {
            hoist().whittle()
//                  ^ d: picker src/main/kotlin/sawhorse/Peg.kt:4, src/main/kotlin/sawhorse/Twig.kt:4
        }
        Twig().run { hoist().whittle() }
//                           ^ d: picker src/main/kotlin/sawhorse/Peg.kt:4, src/main/kotlin/sawhorse/Twig.kt:4
        val sawSpun = object : Sawhorse<Twig>(Twig()) {
            fun sawSpin() {
                hoist().whittle()
//                      ^ d: picker src/main/kotlin/sawhorse/Peg.kt:4, src/main/kotlin/sawhorse/Twig.kt:4
            }
        }
    }

    fun Twig.sawPolish() {
        hoist().whittle()
//              ^ d: picker src/main/kotlin/sawhorse/Peg.kt:4, src/main/kotlin/sawhorse/Twig.kt:4
    }

    fun sawShadowed() {
        fun hoist(): Twig = Twig()
        hoist().whittle()
//              ^ d: picker src/main/kotlin/sawhorse/Peg.kt:4, src/main/kotlin/sawhorse/Twig.kt:4
    }
}

class Bucksaw : Sawhorse<Peg>(Peg()) {
    companion object {
        fun hoist(): Twig = Twig()

        fun sawShelf() {
            hoist().whittle()
//                  ^ d: picker src/main/kotlin/sawhorse/Peg.kt:4, src/main/kotlin/sawhorse/Twig.kt:4
        }
    }
}

class Easel : Stool() {
    fun sawSand() {
        hoist().whittle()
//              ^ d: src/main/kotlin/sawhorse/Peg.kt:4
    }
}

open class Spoke<U>(u: U) : Sawhorse<U>(u) {
    fun tidy() {
        hoist().whittle()
//              ^ d: picker src/main/kotlin/sawhorse/Peg.kt:4, src/main/kotlin/sawhorse/Twig.kt:4
    }
}
