package pins

open class Pump {
    open fun prime() = 0
}

class Bilge : Pump() {
    override fun prime() = 1
    //           ^ d: picker src/main/kotlin/pins/Pump.kt:4
}
