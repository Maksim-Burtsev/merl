package flavors

class Draft(val title: String)

class Saved(val title: String)

class Row(onPick: (item: Draft) -> Unit, item: Saved) {
    val t = item.title
//               ^ d: picker src/main/kotlin/flavors/Gauge.kt:3, src/main/kotlin/flavors/Gauge.kt:5, src/main/kotlin/resources/Labels.kt:20
}

class Picker(
    onPick: (item: Draft) -> Unit,
    item: Saved,
) {
    val t = item.title
//               ^ d: src/main/kotlin/flavors/Gauge.kt:5
}

class Labelled(onPick: (item: Draft) -> Unit) {
    val t = item.title
//               ^ d: picker src/main/kotlin/flavors/Gauge.kt:3, src/main/kotlin/flavors/Gauge.kt:5, src/main/kotlin/resources/Labels.kt:20
}

class Gauge(syncer: Syncer) {
    constructor(flag: Boolean) : this(TODO()) {
        println(syncer.isSyncing)
//                     ^ d: picker src/main/kotlin/flavors/Board.kt:7, src/main/kotlin/flavors/Syncer.kt:4, src/main/kotlin/flavors/Syncer.kt:8
    }

    val one get() = syncer.isSyncing
//                         ^ d: picker src/main/kotlin/flavors/Board.kt:7, src/main/kotlin/flavors/Syncer.kt:4, src/main/kotlin/flavors/Syncer.kt:8

    fun read() = syncer.isSyncing
//                      ^ d: picker src/main/kotlin/flavors/Board.kt:7, src/main/kotlin/flavors/Syncer.kt:4, src/main/kotlin/flavors/Syncer.kt:8
}
