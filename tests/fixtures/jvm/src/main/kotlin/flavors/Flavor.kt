package flavors

enum class Flavor(val suffix: String? = null) {
    demo(".demo"),
    prod,
}

enum class BuildKind(val suffix: String? = null) {
    DEBUG(".debug"),
}

fun configure() {
    Flavor.entries.forEach { flavor ->
        if (flavor.suffix != null) println(flavor.suffix)
//                                                ^ d: src/main/kotlin/flavors/Flavor.kt:3
    }
}

fun one(f: Flavor) = f.suffix
//                     ^ d: src/main/kotlin/flavors/Flavor.kt:3
