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

fun sow(limit: Int): Int {
    class Local(limit: Int) {
        val twice = limit * 2
        //          ^ d: none
    }
    return Local(limit).twice
}
