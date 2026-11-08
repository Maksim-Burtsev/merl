package thisbound

class Gridiron(private val griddle: Griddle) {
    fun sear(): Int = 2

    fun scoped(): Int {
        val a = with(griddle) { sear() }
        //                      ^ d: src/main/kotlin/thisbound/Griddle.kt:4
        val b = griddle.apply { sear() }
        //                      ^ d: src/main/kotlin/thisbound/Griddle.kt:4
        val c = griddle.run { sear() }
        //                    ^ d: src/main/kotlin/thisbound/Griddle.kt:4
        val d = with(griddle) { this.sear() }
        //                           ^ d: src/main/kotlin/thisbound/Griddle.kt:4
        val e = griddle.run { this@Gridiron.sear() }
        //                                  ^ d: src/main/kotlin/thisbound/Gridiron.kt:4
        val f = griddle.also { sear() }
        //                     ^ d: src/main/kotlin/thisbound/Gridiron.kt:4
        val g = griddle.let { sear() }
        //                    ^ d: src/main/kotlin/thisbound/Gridiron.kt:4
        val h = griddle.let { it.sear() }
        //                       ^ d: src/main/kotlin/thisbound/Griddle.kt:4
        val i = Griddle().apply { sear() }
        //                        ^ d: picker src/main/kotlin/thisbound/Gridiron.kt:4, src/main/kotlin/thisbound/Griddle.kt:4
        val j = griddle.run {
            sear()
        //  ^ d: src/main/kotlin/thisbound/Griddle.kt:4
        }
        val k = with(mystery()) { sear() }
        //                        ^ d: picker src/main/kotlin/thisbound/Gridiron.kt:4, src/main/kotlin/thisbound/Griddle.kt:4
        val l = mystery().run { sear() }
        //                      ^ d: picker src/main/kotlin/thisbound/Gridiron.kt:4, src/main/kotlin/thisbound/Griddle.kt:4
        val m = griddles.first().apply { sear() }
        //                               ^ d: picker src/main/kotlin/thisbound/Gridiron.kt:4, src/main/kotlin/thisbound/Griddle.kt:4
        val n = with(Griddle()) { sear() }
        //                        ^ d: picker src/main/kotlin/thisbound/Gridiron.kt:4, src/main/kotlin/thisbound/Griddle.kt:4
        return a + b.sear() + c + d + e + f.sear() + g + h + i.sear() + j + k + l + m.sear() + n
    }

    private val griddles = listOf(griddle)

    private fun mystery() = griddles.random()
}
