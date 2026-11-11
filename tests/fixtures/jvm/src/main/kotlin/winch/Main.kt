package winch

fun run(w: Winch) {
    w.slack.click()
//          ^ d: src/main/kotlin/winch/Pawl.kt:4
    val x = Winch()
    x.slack.click()
//          ^ d: src/main/kotlin/winch/Pawl.kt:4
    w.loose.click()
//          ^ d: picker src/main/kotlin/winch/Pawl.kt:4, src/main/kotlin/winch/Pawl.kt:8
}
