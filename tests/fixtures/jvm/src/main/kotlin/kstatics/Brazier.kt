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
//                    ^ d: picker src/main/kotlin/kstatics/Brazier.kt:6, src/main/kotlin/kstatics/Brazier.kt:22; want src/main/kotlin/kstatics/Brazier.kt:6 (#1489)
    Brazier.Fuel.COAL
//               ^ d: none; want src/main/kotlin/kstatics/Brazier.kt:5 (#1489)
    Brazier.shared.light()
//                 ^ d: picker src/main/kotlin/kstatics/Brazier.kt:9, src/main/kotlin/kstatics/Brazier.kt:25; want src/main/kotlin/kstatics/Brazier.kt:9 (#1489)
    Brazier.make().light()
//                 ^ d: src/main/kotlin/kstatics/Brazier.kt:9
}
