package sauce

open class Saucer {
    fun splash(): Int = 1

    companion object {
        fun make(): Saucer = Saucer()
    }
}
