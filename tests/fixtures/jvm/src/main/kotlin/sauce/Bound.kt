package sauce

// A T bounded by Credenza sees Credenza's property Kilderkin, a Cask (#1514).
fun <T : Credenza> T.decanted(): Int = Kilderkin.make().seethe()
//                                                      ^ d: src/main/kotlin/sauce/Cask.kt:4
