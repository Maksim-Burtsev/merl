package pins

class Gate : Hinge() {
    fun open() = swing()
    //           ^ d: picker src/main/kotlin/pins/a/Hinge.kt:4, src/main/kotlin/pins/b/Hinge.kt:4
}
