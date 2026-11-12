package tubs

class Nested {
    fun e(f: (Tub) -> Unit) = Unit
    inner class Inner {
        fun e(f: (Tun) -> Unit) = Unit
        fun m() = e { it.dunk() }
    //                   ^ d: picker src/main/kotlin/tubs/Hogshead.kt:4, src/main/kotlin/tubs/Tub.kt:4, src/main/kotlin/tubs/Tun.kt:4
    }
    fun n() {
        fun e(f: (Tun) -> Unit) = Unit
        e { it.dunk() }
    //         ^ d: picker src/main/kotlin/tubs/Hogshead.kt:4, src/main/kotlin/tubs/Tub.kt:4, src/main/kotlin/tubs/Tun.kt:4
    }
}
