package rebound

class Cellar {
    private val x = Flagon()

    fun forBlock(es: List<Jug>) { for (x in es) { }; x.glug() }
    //                                                 ^ d: src/main/kotlin/rebound/Flagon.kt:4

    fun bareName(es: List<Jug>) { for (x in es) { }; x.glug() }
    //                                               ^ d: src/main/kotlin/rebound/Cellar.kt:4

    fun caught() { try { } catch (x: Exception) { }; x.glug() }
    //                                                 ^ d: src/main/kotlin/rebound/Flagon.kt:4

    fun bodyBrace(k: Boolean, es: List<Jug>) { if (k) { for (x in es) use(x) } else x.glug() }
    //                                                                                ^ d: src/main/kotlin/rebound/Flagon.kt:4

    fun use(j: Jug) {}
}
