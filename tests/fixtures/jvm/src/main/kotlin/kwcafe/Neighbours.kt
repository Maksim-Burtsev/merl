package kwcafe

fun kwpour(mugful: Int): Int = mugful

fun kwpour(mugful: String): Int = 0

class Kwtray {
    fun kwbrew(mugful: Int = 7): Int = mugful

    fun serve(): Int {
        return kwbrew(mugful = 1)
//                    ^ d: src/main/kotlin/kwcafe/Neighbours.kt:8
    }
}

fun Kwpot.kwlean(): Int {
    return kwbrew(mugful = 1)
//                ^ d: picker src/main/kotlin/kwcafe/Pot.kt:6, …
}

fun kwpours(): Int {
    kwpour(mugful = 1)
//         ^ d: picker src/main/kotlin/kwcafe/Neighbours.kt:3, src/main/kotlin/kwcafe/Neighbours.kt:5
    with(Kwpot()) { kwbrew(mugful = 2) }
//                         ^ d: picker src/main/kotlin/kwcafe/Pot.kt:6, …
    with(Kwpot()) {
        kwbrew(mugful = 3)
//             ^ d: picker src/main/kotlin/kwcafe/Pot.kt:6, …
    }
    val pot = Kwpot()
    pot
        .kwbrew(mugful = 4)
//              ^ d: picker src/main/kotlin/kwcafe/Pot.kt:6, …
    pot.
        kwbrew(mugful = 5)
//             ^ d: picker src/main/kotlin/kwcafe/Pot.kt:6, …
    pot?.kwbrew(mugful = 6)
//              ^ d: picker src/main/kotlin/kwcafe/Pot.kt:6, …
    return 0
}

fun kwsip(mugful: Int): Int = mugful

fun kwsips(): Long {
    fun kwsip(mugful: Long): Long = mugful
    return kwsip(mugful = 7L)
//               ^ d: src/main/kotlin/kwcafe/Neighbours.kt:45
}
