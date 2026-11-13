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
//     ^ d: picker src/main/kotlin/servings/Servings.kt:4, src/main/kotlin/servings/Servings.kt:9, …; want src/main/kotlin/servings/Servings.kt:4 (#1484)
}
