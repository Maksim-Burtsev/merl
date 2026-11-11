@file:Suppress("unused")
//     ^ d: none
package gilded

@Deprecated("use Shelf")
//^ d: none
class Ledge {
    companion object {
        @JvmStatic
        // ^ d: none
        fun make(): Ledge = Ledge()
    }

    @Suppress("MagicNumber")
    // ^ d: none
    fun width(): Int = 3

    @get:JvmStatic
    //    ^ d: none
    val depth: Int = 2
}
