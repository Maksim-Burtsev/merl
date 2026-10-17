package brew

import brew.stale.Samovar as OldSamovar

fun Glass.dip(): Int = dose(1) + scald()
//                     ^ d: picker src/main/kotlin/brew/Glass.kt:4, src/main/kotlin/brew/Glass.kt:6, src/main/kotlin/brew/Glass.kt:8, src/main/kotlin/brew/Samovar.kt:15, src/main/kotlin/brew/stale/Samovar.kt:4; want src/main/kotlin/brew/Glass.kt:4 (#968)
//                               ^ d: picker src/main/kotlin/brew/Glass.kt:10, src/main/kotlin/brew/Samovar.kt:32, src/main/kotlin/brew/Urn.kt:6; want src/main/kotlin/brew/Glass.kt:10 (#968)

fun Samovar.mint(): Int = 0

fun Samovar.soaks(): Int = this.mint() + bulk + this@soaks.dose()
//                              ^ d: picker src/main/kotlin/brew/Table.kt:9, src/main/kotlin/brew/Herbs.kt:4, src/main/kotlin/brew/Samovar.kt:19; want src/main/kotlin/brew/Samovar.kt:19 (#968)
//                                       ^ d: src/main/kotlin/brew/Samovar.kt:11
//                                                         ^ d: picker src/main/kotlin/brew/Glass.kt:4, src/main/kotlin/brew/Glass.kt:6, src/main/kotlin/brew/Glass.kt:8, src/main/kotlin/brew/Samovar.kt:15, src/main/kotlin/brew/stale/Samovar.kt:4; want src/main/kotlin/brew/Samovar.kt:15 (#968)

class Table(val glass: Glass) {
    fun set(samovar: Samovar, old: OldSamovar): Int {
        //                         ^ d: src/main/kotlin/brew/stale/Samovar.kt:3
        val a = glass.dose(1) + glass.dose(1, 2) + glass.dose("two")
        //            ^ d: src/main/kotlin/brew/Glass.kt:4
        //                            ^ d: src/main/kotlin/brew/Glass.kt:6
        //                                               ^ d: src/main/kotlin/brew/Glass.kt:8
        val b = samovar.dose(cups = 2) + samovar.dose() + samovar.mint() + glass.dip()
        //              ^ d: src/main/kotlin/brew/Samovar.kt:15
        //                   ^ d: src/main/kotlin/brew/Samovar.kt:15
        //                                       ^ d: src/main/kotlin/brew/Samovar.kt:15
        //                                                        ^ d: src/main/kotlin/brew/Samovar.kt:19
        //                                                                       ^ d: src/main/kotlin/brew/Table.kt:5
        return a + b + old.dose(3) + Samovar(bulk = 2, vapor = 1).bulk
        //                 ^ d: src/main/kotlin/brew/stale/Samovar.kt:4
        //                                   ^ d: src/main/kotlin/brew/Samovar.kt:11
        //                                                        ^ d: src/main/kotlin/brew/Samovar.kt:11
    }

    fun sift(v: Urn): Int = when (v) {
        is Glass -> v.dose(1)
        //            ^ d: src/main/kotlin/brew/Glass.kt:4
        is Samovar -> v.dose()
        //              ^ d: src/main/kotlin/brew/Samovar.kt:15
        else -> v.scald()
        //        ^ d: picker src/main/kotlin/brew/Glass.kt:10, src/main/kotlin/brew/Samovar.kt:32, src/main/kotlin/brew/Urn.kt:6; want src/main/kotlin/brew/Urn.kt:6 (#969)
    }

    fun hold(x: Any): Int {
        if (x is Glass) return x.dose(2)
        //                       ^ d: src/main/kotlin/brew/Glass.kt:4
        return when (val g = x) {
            is Samovar -> g.mint()
            //              ^ d: picker src/main/kotlin/brew/Table.kt:9, src/main/kotlin/brew/Herbs.kt:4, src/main/kotlin/brew/Samovar.kt:19; want src/main/kotlin/brew/Samovar.kt:19 (#969)
            else -> 0
        }
    }

    fun drape(): Int = Cosy().tilt() + Samovar.mold().bulk + Samovar.CUPS
    //                 ^ d: src/main/kotlin/brew/Cosy.kt:3
    //                        ^ d: src/main/kotlin/brew/Cosy.kt:4
    //                                         ^ d: src/main/kotlin/brew/Samovar.kt:44
    //                                                               ^ d: src/main/kotlin/brew/Samovar.kt:42
}
