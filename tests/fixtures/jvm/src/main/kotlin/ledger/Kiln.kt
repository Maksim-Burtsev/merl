package ledger

class Oven {
    val heat = 1

    object Kiln {
        val heat = 3

        fun warm() = this.heat
        //                ^ d: src/main/kotlin/ledger/Kiln.kt:7
    }
}

fun bake(): String {
    val recipe = """
        val crust = 2
    """
    return recipe
}

val crust = 1

fun crusts() = crust
//             ^ d: src/main/kotlin/ledger/Kiln.kt:21
