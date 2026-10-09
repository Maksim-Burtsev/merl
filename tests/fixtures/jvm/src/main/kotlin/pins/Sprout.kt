package pins

fun sprout(germ: Int): Any {
    class Sprout {
        fun grow() = germ
        //           ^ d: none; want src/main/kotlin/pins/Sprout.kt:3 (#787)
    }
    return Sprout()
}
