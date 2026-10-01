package calls

data class Shelf(
    val sku: String,
)

fun shelved(shelf: Shelf) = shelf.sku
//                                ^ d: src/main/kotlin/calls/Shelf.kt:4
