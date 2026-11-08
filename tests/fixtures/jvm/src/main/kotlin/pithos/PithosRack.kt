package pithos

class PithosRack<Pithos>(val held: Pithos) {
//               ^ d: src/main/kotlin/pithos/PithosRack.kt:3
//                                 ^ d: src/main/kotlin/pithos/PithosRack.kt:3
    fun take(x: Pithos): Pithos = x
    //          ^ d: src/main/kotlin/pithos/PithosRack.kt:3
    //                   ^ d: src/main/kotlin/pithos/PithosRack.kt:3
}

fun <Pithos> pithosOne(x: Pithos): Pithos = x
//   ^ d: src/main/kotlin/pithos/PithosRack.kt:11
//                        ^ d: src/main/kotlin/pithos/PithosRack.kt:11
//                                 ^ d: src/main/kotlin/pithos/PithosRack.kt:11

fun pithosPlain(x: Pithos): Pithos = x
//                 ^ d: src/main/kotlin/pithos/Pithos.kt:3
//                          ^ d: src/main/kotlin/pithos/Pithos.kt:3

class PithosBound<T : Pithos>(val t: T)
//                ^ d: src/main/kotlin/pithos/PithosRack.kt:20
//                                   ^ d: src/main/kotlin/pithos/PithosRack.kt:20
//                    ^ d: src/main/kotlin/pithos/Pithos.kt:3

interface PithosHolder<Pithos> {
//                     ^ d: src/main/kotlin/pithos/PithosRack.kt:25
    fun get(): Pithos
    //         ^ d: src/main/kotlin/pithos/PithosRack.kt:25
}

class PithosOuter<Pithos> {
//                ^ d: src/main/kotlin/pithos/PithosRack.kt:31
    fun <Pithos> inner(x: Pithos): Pithos = x
    //   ^ d: src/main/kotlin/pithos/PithosRack.kt:33
    //                    ^ d: src/main/kotlin/pithos/PithosRack.kt:33
    //                             ^ d: src/main/kotlin/pithos/PithosRack.kt:33
    fun other(x: Pithos): Pithos = x
    //           ^ d: src/main/kotlin/pithos/PithosRack.kt:31
    //                    ^ d: src/main/kotlin/pithos/PithosRack.kt:31
}

typealias PithosPair<Pithos> = List<Pithos>
//                   ^ d: src/main/kotlin/pithos/PithosRack.kt:42
//                                  ^ d: src/main/kotlin/pithos/PithosRack.kt:42
