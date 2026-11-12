package selfalias

class Snuffer {
    var glow = 0
    class Inner { var glow = 0 }
    fun go() {
        val r = object : Runnable {
            var glow = 0
            override fun run() {
                this.glow++
//                   ^ d: src/main/kotlin/selfalias/Snuffer.kt:8
                Wax().apply {
                    this.melt()
//                       ^ d: !jump
                }
                Wax().apply { this.melt() }
//                                 ^ d: !jump
                tally {
                    this.ticks++
//                       ^ d: !jump
                }
            }
            fun melt() {}
            var ticks = 0
            fun Wax.drip() {
                this.melt()
//                   ^ d: !jump
            }
            fun Wax.pour() = this.melt()
//                                ^ d: !jump
        }
    }
}

class Sprocket {
    var teeth = 1
    companion object : Runnable {
        var teeth = 2
        override fun run() {
            this.teeth++
//               ^ d: src/main/kotlin/selfalias/Snuffer.kt:38
        }
        fun Sprocket.whirl() {
            this.teeth++
//               ^ d: picker src/main/kotlin/selfalias/Snuffer.kt:36, src/main/kotlin/selfalias/Snuffer.kt:38
        }
    }
}

class Wax { fun melt() {} }
class Tally { var ticks = 0 }
fun tally(block: Tally.() -> Unit) = Tally().apply(block)
