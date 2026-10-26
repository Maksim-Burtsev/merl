package flavors

class Board(
    syncer: Syncer,
    private val label: String,
) {
    val isSyncing = syncer.isSyncing
//                         ^ d: src/main/kotlin/flavors/Syncer.kt:4
}
