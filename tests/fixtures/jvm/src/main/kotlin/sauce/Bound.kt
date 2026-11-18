package sauce

// A T bounded by Credenza sees Credenza's property Kilderkin, a Cask (#1514).
fun <T : Credenza> T.decanted(): Int = Kilderkin.make().seethe()
//                                                      ^ d: picker src/main/kotlin/sauce/Cask.kt:4, src/main/kotlin/sauce/Kegs.kt:4, src/main/kotlin/sauce/Kegs.kt:12, src/main/kotlin/sauce/Kegs.kt:20; want src/main/kotlin/sauce/Cask.kt:4 (#1515)
