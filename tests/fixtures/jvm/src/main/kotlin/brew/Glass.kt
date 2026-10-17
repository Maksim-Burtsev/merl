package brew

class Glass : Urn() {
    fun dose(cups: Int): Int = cups

    fun dose(cups: Int, lid: Int): Int = cups + lid

    fun dose(cups: String): Int = cups.length

    override fun scald(): Int = super.scald() + tap
    //                                ^ d: src/main/kotlin/brew/Urn.kt:6
    //                                          ^ d: src/main/kotlin/brew/Urn.kt:4
}
