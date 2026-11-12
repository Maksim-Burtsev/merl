package kstatics

class Tumbler {
    fun brew(n: Int): Tumbler {
        return this
    }
    fun pour() {
    }
    companion object {
        fun brew(): Tumbler {
            return Tumbler()
        }
    }
}
