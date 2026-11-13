package sauce

class Ramekin {
    fun splash(): Int = 2

    fun make(): Ramekin = this
}

interface Tagged {
    val label: String
}
