package brew

class Glass : Urn() {
    fun dose(cups: Int): Int = cups

    fun dose(cups: Int, lid: Int): Int = cups + lid

    fun dose(cups: String): Int = cups.length

    override fun scald(): Int = super.scald() + tap
    //                                ^ d: picker src/main/kotlin/brew/Samovar.kt:32, src/main/kotlin/brew/Urn.kt:6; want src/main/kotlin/brew/Urn.kt:6 (#963)
    //                                          ^ d: src/main/kotlin/brew/Urn.kt:4
}
