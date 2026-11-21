package sauce

// run's receiver lambda is a Pour: Saucer is Pour's property, a Ramekin (#1514).
fun ran(pour: Pour): Int = pour.run { Saucer.make().splash() }
//                                                  ^ d: src/main/kotlin/sauce/Ramekin.kt:4
