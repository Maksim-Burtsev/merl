package winch

class Winch {
    private val inner = Pawl()
    val slack: Pawl get() = inner
    var turns: Int = 0
        set(v) {
            field = v
//          ^ d: src/main/kotlin/winch/Winch.kt:6
        }
    val loose get() = Pawl()

    fun grow() {
        slack.click()
//            ^ d: src/main/kotlin/winch/Pawl.kt:4
    }
}
