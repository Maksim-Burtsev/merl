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
            }
        }
    }
}
