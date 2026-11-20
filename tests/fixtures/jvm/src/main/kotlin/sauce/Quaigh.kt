package sauce

// A nested object is a type, not a property of the receiver: inside an extension of Quaigh the
// name Mazer is never read as a value of its supertype Tagged (#1515).
class Quaigh {
    object Mazer : Tagged {
        override val label: String = "m"
    }
}

fun Quaigh.nursed(): String = Mazer.label
//                                  ^ d: src/main/kotlin/sauce/Quaigh.kt:7
