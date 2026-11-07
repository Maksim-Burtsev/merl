package tureens.quoted

object Menu {
  val recipe = """
import tureens.south.Tureen
"""
  def serve(): Int = {
    import tureens.north.Tureen
    new Tureen().ladle()
    //  ^ d: src/main/scala/tureens/North.scala:3
    //           ^ d: src/main/scala/tureens/North.scala:4
  }
}
