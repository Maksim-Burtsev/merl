package kwcafe

fun kwstirs(): Int {
    return kwstir(spoon = 1)
//                ^ d: none
}

fun kwmixer(): Int {
    fun kwmix(spoon: Int): Int = spoon
    return kwmix(spoon = 0)
}

fun kwmixes(): Int {
    return kwmix(spoon = 1)
//               ^ d: none
}

class Kwsugar {
    fun kwmelt(spoon: Int): Int = spoon
}

fun kwmelt(spoon: Int): Int = spoon

context(Kwsugar)
fun kwsweet(): Int {
    return kwmelt(spoon = 1)
//                ^ d: !jump
}

fun kwsplit(): Int {
    val pot = Kwpot()
    pot.
    kwbrew(mugful = 5)
//         ^ d: picker src/main/kotlin/kwcafe/Pot.kt:6, …
    return 0
}
