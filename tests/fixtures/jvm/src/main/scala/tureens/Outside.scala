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
