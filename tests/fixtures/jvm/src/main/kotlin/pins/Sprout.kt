package pins

fun sprout(germ: Int): Any {
    class Sprout {
        fun grow() = germ
        //           ^ d: src/main/kotlin/pins/Sprout.kt:3
    }
    return Sprout()
}

fun shade(germ: Int): Any {
    class Shade {
        val germ = 2
        fun grow() = germ
        //           ^ d: src/main/kotlin/pins/Sprout.kt:13
    }
    return Shade()
}
