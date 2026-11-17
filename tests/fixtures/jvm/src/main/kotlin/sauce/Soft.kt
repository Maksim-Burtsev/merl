package sauce

// assertSoftly comes from a library, and its receiver lambda is the Pour it is given: Saucer is
// Pour's property, a Ramekin (#1514).
fun softly(pour: Pour) = assertSoftly(pour) {
    Saucer.make().splash()
    //            ^ d: picker src/main/kotlin/sauce/Ramekin.kt:4, src/main/kotlin/sauce/Saucer.kt:4; want src/main/kotlin/sauce/Ramekin.kt:4 (#1515)
}
