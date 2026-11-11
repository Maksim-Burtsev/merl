package sawhorse

open class Stool : Sawhorse<Peg>(Peg()) {
    fun tidy() {
        hoist().whittle()
//              ^ d: picker src/main/kotlin/sawhorse/Peg.kt:4, src/main/kotlin/sawhorse/Twig.kt:4; want src/main/kotlin/sawhorse/Peg.kt:4 (#1481)
    }
}

class Easel : Stool()

open class Spoke<U>(u: U) : Sawhorse<U>(u) {
    fun tidy() {
        hoist().whittle()
//              ^ d: picker src/main/kotlin/sawhorse/Peg.kt:4, src/main/kotlin/sawhorse/Twig.kt:4
    }
}
