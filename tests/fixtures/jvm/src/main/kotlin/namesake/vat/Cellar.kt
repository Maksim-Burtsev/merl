package namesake.vat

import namesake.cork.Decanter

class Cellar {
    class Decanter {
        fun aerate() {}
    }

    val kept: Decanter = Decanter()

    fun pour(d: Decanter) {
        d.aerate()
        //^ d: src/main/kotlin/namesake/vat/Cellar.kt:7
        kept.aerate()
        //   ^ d: src/main/kotlin/namesake/vat/Cellar.kt:7
        val c: Decanter = Decanter()
        c.aerate()
        //^ d: src/main/kotlin/namesake/vat/Cellar.kt:7
    }
}

class Rack {
    fun hold(d: Decanter) {
        d.aerate()
        //^ d: src/main/kotlin/namesake/cork/Decanter.kt:4
    }
}
