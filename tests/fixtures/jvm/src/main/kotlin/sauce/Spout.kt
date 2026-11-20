package sauce

// An extension of Trivet sees Trivet's property Pipkin, a Costrel, by its bare name (#1514).
fun Trivet.dribble(): Int = Pipkin.make().seep()
//                                        ^ d: src/main/kotlin/sauce/Pipkin.kt:12
