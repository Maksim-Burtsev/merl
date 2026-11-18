package sauce

// run's receiver lambda is a Pour: Saucer is Pour's property, a Ramekin (#1514).
fun ran(pour: Pour): Int = pour.run { Saucer.make().splash() }
//                                                  ^ d: picker src/main/kotlin/sauce/Ramekin.kt:4, src/main/kotlin/sauce/Saucer.kt:4; want src/main/kotlin/sauce/Ramekin.kt:4 (#1515)
