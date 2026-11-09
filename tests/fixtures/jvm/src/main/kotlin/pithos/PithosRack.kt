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

class PithosRange {
    val size = 3
    fun <Pithos> own(x: Pithos) = x

    // a Pithos counts
//       ^ d: src/main/kotlin/pithos/Pithos.kt:3
    fun check(a: Int): String {
        when {
            a < size -> {
//              ^ d: src/main/kotlin/pithos/PithosRack.kt:47
                return "x" + size
//                           ^ d: src/main/kotlin/pithos/PithosRack.kt:47
            }
        }
        return ""
    }
}

class PithosWrapped<
    Pithos,
//  ^ d: src/main/kotlin/pithos/PithosRack.kt:65
>(
    val held: Pithos,
    //        ^ d: src/main/kotlin/pithos/PithosRack.kt:65
) {
    fun take(x: Pithos): Pithos = x
    //          ^ d: src/main/kotlin/pithos/PithosRack.kt:65
}

fun <Pithos> pithosWrapped(
    x: Pithos,
    // ^ d: src/main/kotlin/pithos/PithosRack.kt:75
): Pithos = x
// ^ d: src/main/kotlin/pithos/PithosRack.kt:75

fun pithosWrappedPlain(
    x: Pithos,
    // ^ d: src/main/kotlin/pithos/Pithos.kt:3
): Pithos = x
// ^ d: src/main/kotlin/pithos/Pithos.kt:3

class PithosWrappedCtor<Pithos>(
    val held: Pithos,
    //        ^ d: src/main/kotlin/pithos/PithosRack.kt:87
)

class PithosCrossBound<
    A : Comparable<Pithos>,
    Pithos,
> {
    fun second(x: Pithos): Pithos = x
    //            ^ d: src/main/kotlin/pithos/PithosRack.kt:94
}
