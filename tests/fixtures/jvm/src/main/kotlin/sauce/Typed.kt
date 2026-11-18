package sauce

// mockk takes its receiver type from the declared type here: Saucer is Pour's property, a
// Ramekin (#1514).
val typed: Pour = mockk {
    Saucer.make().splash()
    //            ^ d: picker src/main/kotlin/sauce/Ramekin.kt:4, src/main/kotlin/sauce/Saucer.kt:4; want src/main/kotlin/sauce/Ramekin.kt:4 (#1515)
}
