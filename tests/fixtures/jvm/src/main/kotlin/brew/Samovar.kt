package brew

import brew.Herbs.mint
import brew.Herbs.sage

/**
 * class Glass {
 * fun dose(cups: Int): Int = 0
 * See [Glass.dose].
 */
class Samovar(vapor: Int, val bulk: Int = 1) : Urn() {
    private val vapor: Int = vapor
    //                       ^ d: src/main/kotlin/brew/Samovar.kt:11

    fun dose(cups: Int = 1): Int = cups + vapor
    //                             ^ d: src/main/kotlin/brew/Samovar.kt:15
    //                                    ^ d: src/main/kotlin/brew/Samovar.kt:12

    fun mint(): Int = 7

    fun treat(): Int {
        val vapor = 1
        return vapor + this.vapor + super.tap + dose(2) + mint() + sage()
        //     ^ d: src/main/kotlin/brew/Samovar.kt:22
        //                  ^ d: src/main/kotlin/brew/Samovar.kt:12
        //                                ^ d: src/main/kotlin/brew/Urn.kt:4
        //                                      ^ d: src/main/kotlin/brew/Samovar.kt:15
        //                                                ^ d: src/main/kotlin/brew/Samovar.kt:19
        //                                                         ^ d: src/main/kotlin/brew/Herbs.kt:6
    }

    override fun scald(): Int = super.scald() + Herbs.mint() + Cosy().tilt()
    //                                ^ d: picker src/main/kotlin/brew/Glass.kt:10, src/main/kotlin/brew/Urn.kt:6; want src/main/kotlin/brew/Urn.kt:6 (#963)
    //                                                ^ d: src/main/kotlin/brew/Herbs.kt:4
    //                                                         ^ d: src/main/kotlin/brew/Samovar.kt:37
    //                                                                ^ d: picker src/main/kotlin/brew/Samovar.kt:38, src/main/kotlin/brew/Cosy.kt:4; want src/main/kotlin/brew/Samovar.kt:38 (#965)
    class Cosy {
        fun tilt(): Int = 2
    }

    companion object {
        const val CUPS = 4

        fun mold(): Samovar = Samovar(CUPS)
        //                    ^ d: src/main/kotlin/brew/Samovar.kt:11
        //                            ^ d: src/main/kotlin/brew/Samovar.kt:42
    }

    fun pourAll(glasses: List<Glass>): Int {
        val total = glasses.sumOf { it.dose(1) + this@Samovar.vapor }
        //                             ^ d: picker src/main/kotlin/brew/Glass.kt:4, src/main/kotlin/brew/Glass.kt:6, src/main/kotlin/brew/Glass.kt:8; want src/main/kotlin/brew/Glass.kt:4 (#964)
        //                                                    ^ d: src/main/kotlin/brew/Samovar.kt:12
        return glasses.sumOf { vapor -> vapor.dose("x") } + total
        //                              ^ d: src/main/kotlin/brew/Samovar.kt:53
        //                                    ^ d: picker src/main/kotlin/brew/Glass.kt:4, src/main/kotlin/brew/Glass.kt:6, src/main/kotlin/brew/Glass.kt:8; want src/main/kotlin/brew/Glass.kt:8 (#964)
    }

    fun ritual(): String = """
        class Glass {
            fun dose(cups: Int): Int = 0
        }
    """ + "fun mint(): Int = $vapor"
    //                         ^ d: src/main/kotlin/brew/Samovar.kt:12
}
