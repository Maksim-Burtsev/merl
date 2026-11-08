package kwcafe

fun serve(): Int {
    val mugful = 5
    val pot = Kwpot()
    kwbrew(mugful = mugful)
//         ^ d: picker src/main/kotlin/kwcafe/Brewer.kt:5, …
//                  ^ d: src/main/kotlin/kwcafe/Uses.kt:4
    pot.kwbrew(mugful = 4)
//             ^ d: picker src/main/kotlin/kwcafe/Pot.kt:6, …
    kwbrew(mugful)
//         ^ d: src/main/kotlin/kwcafe/Uses.kt:4
    return 0
}
