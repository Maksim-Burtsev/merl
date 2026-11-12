package flax

object Retter {
    fun scutch(): Int = 1

    fun oneLine(): Int { fun scutch(): Int = 2; return scutch() }
    //                                                 ^ d: src/main/kotlin/flax/Retter.kt:6

    fun below(): Int {
        fun scutch(): Int = 3
        return scutch()
//             ^ d: src/main/kotlin/flax/Retter.kt:10
    }

    fun fact(n: Int): Int {
        fun scutch(k: Int): Int =
            if (k == 0) 1 else scutch(k - 1)
        //                     ^ d: src/main/kotlin/flax/Retter.kt:16
        return scutch(n)
//             ^ d: src/main/kotlin/flax/Retter.kt:16
    }

    fun braced(n: Int): Int {
        fun scutch(k: Int): Int {
            return if (k == 0) 1 else scutch(k - 1)
//                                    ^ d: src/main/kotlin/flax/Retter.kt:24
        }
        return scutch(n)
//             ^ d: src/main/kotlin/flax/Retter.kt:24
    }

    fun inner(n: Int): Int {
        if (n > 1) {
            fun scutch(): Int = 6
            return scutch()
//                 ^ d: src/main/kotlin/flax/Retter.kt:34
        }
        return scutch()
//             ^ d: src/main/kotlin/flax/Retter.kt:4
    }

    fun plain(): Int = scutch()
    //                 ^ d: src/main/kotlin/flax/Retter.kt:4
}

class Brake {
    fun scutch(): Int = 7

    fun fact(n: Int): Int {
        fun scutch(k: Int): Int =
            if (k == 0) 1 else scutch(k - 1)
        //                     ^ d: src/main/kotlin/flax/Retter.kt:50
        return scutch(n)
//             ^ d: src/main/kotlin/flax/Retter.kt:50
    }

    fun later(): Int = scutch()
    //                 ^ d: src/main/kotlin/flax/Retter.kt:47

    fun dotted(n: Int): Int {
        fun scutch(k: Int): Int =
            k + this.scutch()
        //            ^ d: src/main/kotlin/flax/Retter.kt:47
        return scutch(n)
    }

    fun member(): Int {
        val o = object {
            fun hackle(): Int = 1
        }
        return o.hackle() + scutch()
//                          ^ d: src/main/kotlin/flax/Retter.kt:47
    }
}

fun twice(n: Int): Int {
    fun hurl(k: Int): Int = k; return hurl(hurl(n))
    //                                ^ d: src/main/kotlin/flax/Retter.kt:77
}

class Bolter(val sley: String) {
    fun render(): String {
        fun sley(): String {
            return sley.uppercase()
//                 ^ d: src/main/kotlin/flax/Retter.kt:81
        }
        return sley()
//             ^ d: src/main/kotlin/flax/Retter.kt:83
    }

    fun oneLine(): String { fun sley(): String = "x"; return sley + sley() }
    //                                                       ^ d: src/main/kotlin/flax/Retter.kt:81
    //                                                              ^ d: src/main/kotlin/flax/Retter.kt:91
}
