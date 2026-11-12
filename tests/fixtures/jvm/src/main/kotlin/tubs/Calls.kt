package tubs

class Calls(private val x: Tun) {
    private fun e(f: (Tub) -> Unit) = Unit
    private fun <T> each(t: T, f: (T) -> Unit) = Unit
    fun a() {
        e { x ->
            x.dunk()
    //        ^ d: src/main/kotlin/tubs/Tub.kt:4
        }
    }
    fun b() = e { x -> x.dunk() }
    //                   ^ d: src/main/kotlin/tubs/Tub.kt:4
    fun c() {
        e {
            it.slop()
    //         ^ d: src/main/kotlin/tubs/Tub.kt:5
        }
    }
    fun d() = e { run { it.dunk() } }
    //                     ^ d: picker src/main/kotlin/tubs/Hogshead.kt:4, src/main/kotlin/tubs/Tub.kt:4, src/main/kotlin/tubs/Tun.kt:4
    fun g(e: ((Tun) -> Unit) -> Unit) = e { it.dunk() }
    //                                         ^ d: picker src/main/kotlin/tubs/Hogshead.kt:4, src/main/kotlin/tubs/Tub.kt:4, src/main/kotlin/tubs/Tun.kt:4
    fun h() = each(Tun()) { it.dunk() }
    //                         ^ d: picker src/main/kotlin/tubs/Hogshead.kt:4, src/main/kotlin/tubs/Tub.kt:4, src/main/kotlin/tubs/Tun.kt:4
    fun i() = x.dunk()
    //          ^ d: src/main/kotlin/tubs/Tun.kt:4
    fun j() = e { y -> x.dunk() }
    //                   ^ d: src/main/kotlin/tubs/Tun.kt:4
    fun k() {
        e { x ->
            val x = Tun()
            x.dunk()
    //        ^ d: src/main/kotlin/tubs/Tun.kt:4
        }
    }
    fun r() = e { x -> val x = Tun(); x.dunk() }
    //                                  ^ d: src/main/kotlin/tubs/Tun.kt:4
}
