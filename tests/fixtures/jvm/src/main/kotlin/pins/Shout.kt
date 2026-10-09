package pins

fun CharSequence.shout() = this

fun String.whisper() = this

fun greet(s: String) {
    s.shout()
    //^ d: src/main/kotlin/pins/Shout.kt:3
    s.whisper()
    //^ d: src/main/kotlin/pins/Shout.kt:5
}
