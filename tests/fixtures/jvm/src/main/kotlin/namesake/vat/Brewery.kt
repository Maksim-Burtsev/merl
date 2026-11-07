package namesake.vat

import namesake.cork.Ewer
import namesake.cork.Funnel

class Brewery {
    object Funnel {
        fun strain() {}
    }

    class Ewer {
        companion object {
            fun brim(): Ewer = Ewer()
        }
    }

    fun brew() {
        Funnel.strain()
        //     ^ d: src/main/kotlin/namesake/vat/Brewery.kt:8
        Ewer.brim()
        //   ^ d: src/main/kotlin/namesake/vat/Brewery.kt:13
        Funnel
            .strain()
        //   ^ d: src/main/kotlin/namesake/vat/Brewery.kt:8
    }
}

fun spill() {
    Funnel.strain()
    //     ^ d: src/main/kotlin/namesake/cork/Funnel.kt:4
}
