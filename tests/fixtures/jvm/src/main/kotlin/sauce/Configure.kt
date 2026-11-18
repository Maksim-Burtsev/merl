package sauce

// configure is a library extension whose receiver lambda is the Pour it is called on: Saucer is
// Pour's property, a Ramekin (#1514).
fun configured(pour: Pour) = pour.configure {
    Saucer.make().splash()
    //            ^ d: picker src/main/kotlin/sauce/Ramekin.kt:4, src/main/kotlin/sauce/Saucer.kt:4; want src/main/kotlin/sauce/Ramekin.kt:4 (#1515)
}
