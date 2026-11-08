package tureens.outside

import tureens.south.Tureen

class Shelf {
  val kept: Tureen = new Tureen
  def f(): Int = {
    import tureens.north.Tureen
    kept.ladle()
    //   ^ d: src/main/scala/tureens/South.scala:4
  }
  def g(t: Tureen): Int = {
    import tureens.north.Tureen
    t.ladle()
    //^ d: src/main/scala/tureens/South.scala:4
  }
}

class Crock extends Tureen

object Larder {
  def h(): Int = {
    import tureens.north.Tureen
    new Crock().ladle()
    //          ^ d: src/main/scala/tureens/South.scala:4
  }
}

class Cellar {
  import tureens.north.Tureen
  val held: Tureen = new Tureen
  def make(): Tureen = new Tureen
}

object Stair {
  def k(c: Cellar): Int = {
    c.held.ladle()
    //     ^ d: src/main/scala/tureens/North.scala:4
    new Cellar().held.ladle()
    //                ^ d: picker src/main/scala/tureens/North.scala:4, src/main/scala/tureens/South.scala:4
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
