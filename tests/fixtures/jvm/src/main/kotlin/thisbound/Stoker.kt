package thisbound

class Hearth {
    fun stoke(): Int = 1
}

// In an extension declared in an interface, this@stoked is the extension's receiver, a Hearth,
// not the interface (#1502).
interface Stoker {
    fun stoke(): Int

    suspend fun Hearth.stoked() = this@stoked.stoke()
    //                                        ^ d: src/main/kotlin/thisbound/Stoker.kt:4
}
