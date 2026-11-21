package app

// A generic receiver lambda (T.() -> Unit) may be a Cellar: Gourd is Cellar's property, a Flute
// (#1514). It has a project of its own, as it keeps every class-named property in play.
fun <T> T.tap(block: T.() -> Unit): T {
    block()
    return this
}

fun tapped(c: Cellar): Cellar = c.tap { Gourd.make().drip() }
//                                                   ^ d: kt/src/main/kotlin/app/Gourd.kt:12
