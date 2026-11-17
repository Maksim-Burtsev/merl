package sauce

// apply's receiver lambda is a Pour: Saucer is Pour's property, a Ramekin (#1514).
fun applied(pour: Pour): Pour = pour.apply { Saucer.make().splash() }
//                                                         ^ d: picker src/main/kotlin/sauce/Ramekin.kt:4, src/main/kotlin/sauce/Saucer.kt:4; want src/main/kotlin/sauce/Ramekin.kt:4 (#1515)
