package kwcafe

fun kwelvis(cached: Int?): Int {
    return cached ?: kwbrew(mugful = 1)
//                          ^ d: src/main/kotlin/kwcafe/Brewer.kt:5
}

class Kwfunctor {
    operator fun invoke(cups: Int): Int = cups
}

fun kwsteep(cups: Int): Int = cups

fun kwsteeps(kwsteep: Kwfunctor): Int {
    return kwsteep(cups = 1)
//                 ^ d: none
}

fun kwdrip(
    cups: Int,
    mugful: Int = 0,
): Int = cups + mugful

fun kwdrips(): Int {
    return kwdrip(cups = 1, mugful = 2)
//                          ^ d: src/main/kotlin/kwcafe/Guards.kt:21
}

fun kwpourer(cups: Int): Int = cups
