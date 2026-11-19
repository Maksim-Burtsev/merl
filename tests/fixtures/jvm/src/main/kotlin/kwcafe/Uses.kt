package kwcafe

fun serve(): Int {
    val mugful = 5
    val pot = Kwpot()
    kwbrew(mugful = mugful)
//         ^ d: src/main/kotlin/kwcafe/Brewer.kt:5
//                  ^ d: src/main/kotlin/kwcafe/Uses.kt:4
    kwbrew(
        mugful = 6,
//      ^ d: src/main/kotlin/kwcafe/Brewer.kt:5
    )
    pot.kwbrew(mugful = 4)
//             ^ d: picker src/main/kotlin/kwcafe/Pot.kt:6, …
    kwbrew(mugful)
//         ^ d: src/main/kotlin/kwcafe/Uses.kt:4
    val kettle = Kwkettle()
    kettle.kwboil(mugful = 4)
//                ^ d: src/main/kotlin/kwcafe/Kettle.kt:4
    return 0
}
