package refill

class Shelf {
    fun branch(c: Boolean) {
        val z = Cruet()
        val q = if (c) { val z = Phial(); z.tipple() } else z.tipple()
        //                                  ^ d: picker src/main/kotlin/refill/Phial.kt:4, …
        //                                                    ^ d: src/main/kotlin/refill/Cruet.kt:4
        z.tipple()
        //^ d: src/main/kotlin/refill/Cruet.kt:4
    }

    fun closed() {
        val z = Cruet()
        run { val z = Phial() }; z.tipple()
        //                         ^ d: src/main/kotlin/refill/Cruet.kt:4
    }

    fun quotedHeader() {
        val z = Cruet()
        println("for (z in all)"); z.tipple()
        //                           ^ d: src/main/kotlin/refill/Cruet.kt:4
    }
}
