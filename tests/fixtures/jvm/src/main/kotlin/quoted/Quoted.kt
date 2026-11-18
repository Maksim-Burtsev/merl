package quoted

class Stylus {
    fun blot(): String = "stylus"
}

class Nib {
    fun blot(): String = "nib"
}

fun a(s: Stylus): String = "s.blot() ${s.blot()}"
//                            ^ d: picker src/main/kotlin/quoted/Quoted.kt:4, src/main/kotlin/quoted/Quoted.kt:8, …
//                                       ^ d: src/main/kotlin/quoted/Quoted.kt:4

fun b(s: Stylus): String = """
    s.blot() ${s.blot()}
//    ^ d: picker src/main/kotlin/quoted/Quoted.kt:4, src/main/kotlin/quoted/Quoted.kt:8, …
//                ^ d: src/main/kotlin/quoted/Quoted.kt:4
"""

fun c(s: Stylus): String = """
    x
""" + s.blot()
//      ^ d: src/main/kotlin/quoted/Quoted.kt:4

fun ink(
    s: Stylus,
): String = s.blot()
//            ^ d: src/main/kotlin/quoted/Quoted.kt:4

object Daos {
    fun ink(
        s: Stylus,
    ): String = s.blot()
    //            ^ d: src/main/kotlin/quoted/Quoted.kt:4
}

class Repo(private val s: Stylus) {
    fun query(s: Nib): String = """
        SELECT 1
    """.trimIndent()

    fun run(): String = s.blot()
    //                    ^ d: src/main/kotlin/quoted/Quoted.kt:4
}
