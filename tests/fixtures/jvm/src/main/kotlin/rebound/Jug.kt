package rebound

class Jug {
    fun glug() {}

    fun anon() = object : Flagon() { fun z() { this.glug() } }
    //                                              ^ d: picker src/main/kotlin/rebound/Flagon.kt:4, …

    fun after() { this.glug() }
    //                 ^ d: src/main/kotlin/rebound/Jug.kt:4
}
