package app

open class Gourd {
    fun drip(): Int = 1

    companion object {
        fun make(): Gourd = Gourd()
    }
}

class Flute {
    fun drip(): Int = 2

    fun make(): Flute = this
}
