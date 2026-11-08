package namesake.cork

object Funnel {
    fun strain() {}
}

class Ewer {
    companion object {
        fun brim(): Ewer = Ewer()
    }
}
