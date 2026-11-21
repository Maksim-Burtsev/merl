package sauce

// infuse's block takes a Settle as its receiver: Piggin is Settle's property, a Cask (#1514).
fun infuse(block: Settle.() -> Unit) = Settle().block()

fun infused() = infuse { Piggin.make().seethe() }
//                                     ^ d: src/main/kotlin/sauce/Cask.kt:4
