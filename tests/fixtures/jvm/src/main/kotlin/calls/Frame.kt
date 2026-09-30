package calls

data class Frame(val wide: Int, val tall: Int)

fun area(f: Frame) = f.wide * f.tall
//                              ^ d: src/main/kotlin/calls/Frame.kt:3
