package burette

class Ampoule {
    val stopcock: Int = 1
}

class Burette {
    val stopcock: Int = 2
}

class Dropper {
    fun dispense(): Burette = Burette()
    fun dispensePair(): Pair<Burette, Ampoule> = Pair(Burette(), Ampoule())
}

fun dispense(): Ampoule = Ampoule()

fun dispensePair(): Pair<Ampoule, Burette> = Pair(Ampoule(), Burette())

fun dropping(block: Dropper.() -> Unit) = Dropper().block()
