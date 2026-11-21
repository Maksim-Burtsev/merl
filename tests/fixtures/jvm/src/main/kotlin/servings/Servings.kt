package servings

class Crock {
    fun stew() {
    }
}

class Wok {
    fun stew() {
    }
}

data class Kit(val crock: Crock, val wok: Wok)

fun servePair(): Pair<Crock, Wok> = Pair(Crock(), Wok())

fun serveKit(): Kit = Kit(Crock(), Wok())

fun unpack() {
    val (c, w) = servePair()
    w.stew()
//    ^ d: src/main/kotlin/servings/Servings.kt:9
    c.stew()
//    ^ d: src/main/kotlin/servings/Servings.kt:4
    val (kc, kw) = serveKit()
    kw.stew()
//     ^ d: src/main/kotlin/servings/Servings.kt:9
    val k = serveKit()
    k.wok.stew()
//        ^ d: src/main/kotlin/servings/Servings.kt:9
}

fun serveTriple(): Triple<Crock, Wok, Crock> = Triple(Crock(), Wok(), Crock())

fun serveNested(): Pair<Crock, List<Wok>> = Pair(Crock(), listOf(Wok()))

fun unpackMore() {
    val (t1, t2, t3) = serveTriple()
    t3.stew()
//     ^ d: src/main/kotlin/servings/Servings.kt:4
    t2.stew()
//     ^ d: src/main/kotlin/servings/Servings.kt:9
    val (n1) = serveNested()
    n1.stew()
//     ^ d: src/main/kotlin/servings/Servings.kt:4
}

fun unpackMade(kits: List<Kit>) {
    val (e, f) = Crock() to Wok()
    f.stew()
//    ^ d: src/main/kotlin/servings/Servings.kt:9
    e.stew()
//    ^ d: src/main/kotlin/servings/Servings.kt:4
    kits.forEach { (g, h) -> h.stew() }
//                             ^ d: src/main/kotlin/servings/Servings.kt:9
}

fun unpackMap(trays: Map<Wok, Kit>) {
    trays.forEach { (wok, kit) -> wok.stew() }
//                                    ^ d: picker src/main/kotlin/servings/Servings.kt:4, src/main/kotlin/servings/Servings.kt:9, …
}

data class Skillet(val wok: Wok, val crock: Crock)

// Two destructuring lambdas on one line: a name is typed by the innermost one around it that
// declares it, never by the first on the line (#1610).
fun unpackNested(kits: List<Kit>, skillets: List<Skillet>) {
    kits.forEach { (a, b) -> skillets.forEach { (a, b) -> a.stew() } }
//                                                          ^ d: src/main/kotlin/servings/Servings.kt:9
    kits.forEach { (a, b) -> skillets.forEach { (c, d) -> a.stew() } }
//                                                          ^ d: src/main/kotlin/servings/Servings.kt:4
    kits.forEach { (a, b) -> skillets.forEach { (a, b) -> b.stew() }; a.stew() }
//                                                                      ^ d: src/main/kotlin/servings/Servings.kt:4
    kits.forEach { (a, b) -> skillets.forEach { a -> a.stew() } }
//                                                     ^ d: !jump
}
