package pins

class Ink(val pigment: Int)

class Paint(val pigment: Int)

class Brush {
    val coat: Ink = Ink(1)

    companion object {
        val coat: Paint = Paint(2)
    }

    fun shade() = coat.pigment
    //                 ^ d: picker src/main/kotlin/pins/Paints.kt:3, src/main/kotlin/pins/Paints.kt:5

    fun dab() = 1
    //  ^ d: src/main/kotlin/pins/Paints.kt:17

    val r = object : Runnable {
        override fun run() {
            gloss
            //^ d: src/main/kotlin/pins/Paints.kt:27
            //status: local
        }

        val gloss = 1
    }

    fun spin(n: Int): Int = if (n > 0) spin(n - 1) else 0
    //                                 ^ d: src/main/kotlin/pins/Paints.kt:30
}
