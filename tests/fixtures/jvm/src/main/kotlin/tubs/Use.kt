package tubs

class Use(private val tubs: List<Tub>, private val c: Tun) {
    fun a() = tubs.forEach { it.dunk() }
    //                          ^ d: src/main/kotlin/tubs/Tub.kt:4
    fun b() = tubs.map { c -> c.dunk() }
    //                          ^ d: src/main/kotlin/tubs/Tub.kt:4
    fun e(f: (Tub) -> Unit) = Unit
    fun g() = e { it.dunk() }
    //               ^ d: src/main/kotlin/tubs/Tub.kt:4
    fun g2() = e { x -> x.dunk() }
    //                    ^ d: src/main/kotlin/tubs/Tub.kt:4
    fun h() = tubs.sortedBy { it.slop() }
    //                           ^ d: src/main/kotlin/tubs/Tub.kt:5
    fun i(more: List<Tub>) = more.filter { x -> x.dunk() }
    //                                            ^ d: src/main/kotlin/tubs/Tub.kt:4
    fun j() {
        val local: List<Tub> = tubs
        local.forEach { x -> x.dunk() }
        //                     ^ d: src/main/kotlin/tubs/Tub.kt:4
    }
    fun k() = tubs.sortedWith { a, b -> a.slop() - b.slop() }
    //                                    ^ d: src/main/kotlin/tubs/Tub.kt:5
    //                                               ^ d: src/main/kotlin/tubs/Tub.kt:5
    fun k2(byName: Map<String, Tub>) = byName.forEach { k, v -> v.dunk() }
    //                                                            ^ d: !jump
    fun k3(tuns: List<Tun>) = tubs.sortedWith { a, b -> tuns.maxOf { a.slop() } }
    //                                                                 ^ d: !jump
    fun k4() = tubs.sortedWith { a, b -> val a = c; a.slop() - b.slop() }
    //                                                ^ d: !jump
    //                                                           ^ d: src/main/kotlin/tubs/Tub.kt:5
    fun k5(tuns: List<Tun>) = tuns.sortedWith { a, b -> b.slop() - a.slop() }
    //                                                    ^ d: src/main/kotlin/tubs/Tun.kt:5
    //                                                               ^ d: src/main/kotlin/tubs/Tun.kt:5
    fun k6() = tubs.sortedBy { a, b -> a.slop() }
    //                                   ^ d: !jump
    fun l(more: List<Tub>) = more.forEach { c -> c.dunk() }
    //                                             ^ d: src/main/kotlin/tubs/Tub.kt:4
    fun m() = c.dunk()
    //          ^ d: src/main/kotlin/tubs/Tun.kt:4
    fun n() = tubs.firstOrNull { x -> x.dunk() }
    //                                  ^ d: src/main/kotlin/tubs/Tub.kt:4
    fun o(more: Array<Tub>) = more.forEach { it.dunk() }
    //                                          ^ d: src/main/kotlin/tubs/Tub.kt:4
    fun p() = tubs.forEach { x -> run { val x = Tun(); x.dunk() } }
    //                                                   ^ d: src/main/kotlin/tubs/Tun.kt:4
    fun q() {
        tubs.forEach { x ->
            val x = Tun()
            x.dunk()
    //        ^ d: src/main/kotlin/tubs/Tun.kt:4
        }
    }
    fun r() = tubs.forEach {
        it.dunk()
    //     ^ d: src/main/kotlin/tubs/Tub.kt:4
    }
}
