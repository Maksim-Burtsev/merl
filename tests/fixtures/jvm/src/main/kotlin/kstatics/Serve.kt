package kstatics

fun serve(t: Tumbler) {
    Tumbler.brew()
//          ^ d: src/main/kotlin/kstatics/Tumbler.kt:10
    Flask.brew()
//        ^ d: src/main/kotlin/kstatics/Flask.kt:7
    t.brew(2)
//    ^ d: src/main/kotlin/kstatics/Tumbler.kt:4
    Tumbler.brew().pour()
//                 ^ d: src/main/kotlin/kstatics/Tumbler.kt:7
    Flask.brew().pour()
//               ^ d: src/main/kotlin/kstatics/Flask.kt:4
}
