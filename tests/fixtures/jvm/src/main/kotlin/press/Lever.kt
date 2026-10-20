package press

interface Lever {
    fun raise(): Int = 1

    fun lower(): Int
}
