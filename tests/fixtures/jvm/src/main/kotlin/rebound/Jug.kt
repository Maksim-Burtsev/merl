package rebound

class Jug {
    fun glug() {}

    fun anon() = object : Flagon() { fun z() { this.glug() } }
    //                                              ^ d: picker src/main/kotlin/rebound/Flagon.kt:4, …

    fun after() { this.glug() }
    //                 ^ d: src/main/kotlin/rebound/Jug.kt:4

    fun bareLine() = object : Flagon() { fun z() { glug() } }
    //                                                ^ d: src/main/kotlin/rebound/Flagon.kt:4

    fun bareAfter() { object : Flagon() { fun z() {} }; glug() }
    //                                                   ^ d: src/main/kotlin/rebound/Jug.kt:4

    fun bareLines() = object : Flagon() {
        fun z() { glug() }
        //        ^ d: src/main/kotlin/rebound/Flagon.kt:4
    }
}
