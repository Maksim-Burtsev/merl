package sauce

// infuse's block takes a Settle as its receiver: Piggin is Settle's property, a Cask (#1514).
fun infuse(block: Settle.() -> Unit) = Settle().block()

fun infused() = infuse { Piggin.make().seethe() }
//                                     ^ d: picker src/main/kotlin/sauce/Cask.kt:4, src/main/kotlin/sauce/Kegs.kt:4, src/main/kotlin/sauce/Kegs.kt:12, src/main/kotlin/sauce/Kegs.kt:20; want src/main/kotlin/sauce/Cask.kt:4 (#1515)
