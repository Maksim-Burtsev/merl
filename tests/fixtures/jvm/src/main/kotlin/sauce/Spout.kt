package sauce

// An extension of Trivet sees Trivet's property Pipkin, a Costrel, by its bare name (#1514).
fun Trivet.dribble(): Int = Pipkin.make().seep()
//                                        ^ d: picker src/main/kotlin/sauce/Pipkin.kt:4, src/main/kotlin/sauce/Pipkin.kt:12; want src/main/kotlin/sauce/Pipkin.kt:12 (#1515)
