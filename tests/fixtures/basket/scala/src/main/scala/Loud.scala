import Rich._

object Loud {
  def run(): String = {
    val text = "a,b"
    text.holler()
    //   ^ d: scala/src/main/scala/Rich.scala:3
  }
}
