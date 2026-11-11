package kstatics

class Brazier {
    enum class Fuel {
        COAL, WOOD;
        fun smoulder() {
        }
    }
    fun light() {
    }
    companion object {
        val shared = Brazier()
        fun make(): Brazier {
            return Brazier()
        }
    }
}

class Grate {
    enum class Fuel {
        COAL;
        fun smoulder() {
        }
    }
    fun light() {
    }
}

fun stoke() {
    Brazier.Fuel.COAL.smoulder()
//                    ^ d: src/main/kotlin/kstatics/Brazier.kt:6
    Brazier.Fuel.COAL
//               ^ d: src/main/kotlin/kstatics/Brazier.kt:5
    Brazier.shared.light()
//                 ^ d: src/main/kotlin/kstatics/Brazier.kt:9
    Brazier.make().light()
//                 ^ d: src/main/kotlin/kstatics/Brazier.kt:9
}

object Hearth {
    val shared = Brazier()
    val ash = Grate()
}

fun rake() {
    Grate.Fuel.COAL.smoulder()
//                  ^ d: src/main/kotlin/kstatics/Brazier.kt:22
    Grate.Fuel.COAL
//             ^ d: src/main/kotlin/kstatics/Brazier.kt:21
    Hearth.shared.light()
//                ^ d: src/main/kotlin/kstatics/Brazier.kt:9
    Hearth.ash.light()
//             ^ d: src/main/kotlin/kstatics/Brazier.kt:25
}
