package flavors

interface Syncer {
    val isSyncing: Boolean
}

class Sleeper {
    val isSyncing = false
}
