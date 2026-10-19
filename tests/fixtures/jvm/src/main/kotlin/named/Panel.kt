package named

fun panel() {
    Knob(onClick = {}, padding = 2)
//  ^ d: src/main/kotlin/named/Knob.kt:3
    Knob(onClick = {}, label = "a")
//  ^ d: picker src/main/kotlin/named/Knob.kt:3, src/main/kotlin/named/Knob.kt:10
}
