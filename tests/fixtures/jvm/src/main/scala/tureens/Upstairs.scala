package tureens.upstairs

import tureens.south.Tureen
import tureens.outside.Cellar

object Landing {
  def k(c: Cellar): Int = {
    c.held.ladle()
    //     ^ d: src/main/scala/tureens/North.scala:4
    c.make().ladle()
    //       ^ d: picker src/main/scala/tureens/North.scala:4, src/main/scala/tureens/South.scala:4
    val made = c.make()
    made.ladle()
    //   ^ d: picker src/main/scala/tureens/North.scala:4, src/main/scala/tureens/South.scala:4
    val got = c.held
    got.ladle()
    //  ^ d: picker src/main/scala/tureens/North.scala:4, src/main/scala/tureens/South.scala:4
  }
}
