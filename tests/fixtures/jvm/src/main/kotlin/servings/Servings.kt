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
//    ^ d: picker src/main/kotlin/servings/Servings.kt:4, src/main/kotlin/servings/Servings.kt:9, …; want src/main/kotlin/servings/Servings.kt:9 (#1484)
    c.stew()
//    ^ d: picker src/main/kotlin/servings/Servings.kt:4, src/main/kotlin/servings/Servings.kt:9, …; want src/main/kotlin/servings/Servings.kt:4 (#1484)
    val (kc, kw) = serveKit()
    kw.stew()
//     ^ d: picker src/main/kotlin/servings/Servings.kt:4, src/main/kotlin/servings/Servings.kt:9, …; want src/main/kotlin/servings/Servings.kt:9 (#1484)
    val k = serveKit()
    k.wok.stew()
//        ^ d: picker src/main/kotlin/servings/Servings.kt:4, src/main/kotlin/servings/Servings.kt:9, …; want src/main/kotlin/servings/Servings.kt:9 (#1484)
}
