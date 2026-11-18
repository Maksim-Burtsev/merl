package sauce

open class Rhyton {
    fun gurgle(): Int = 5

    companion object {
        fun make(): Rhyton = Rhyton()
    }
}

class Situla {
    fun gurgle(): Int = 6

    fun make(): Situla = this
}
