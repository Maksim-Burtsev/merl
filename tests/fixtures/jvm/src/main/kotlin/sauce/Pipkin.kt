package sauce

open class Pipkin {
    fun seep(): Int = 3

    companion object {
        fun make(): Pipkin = Pipkin()
    }
}

class Costrel {
    fun seep(): Int = 4

    fun make(): Costrel = this
}
