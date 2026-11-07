package tureens.blocks

object Kitchen {
  def first(): Int = {
    import tureens.south.Tureen
    new Tureen().ladle()
    //  ^ d: src/main/scala/tureens/South.scala:3
    //           ^ d: src/main/scala/tureens/South.scala:4
  }
  def second(): Int = {
    import tureens.north.Tureen
    new Tureen().ladle()
    //  ^ d: src/main/scala/tureens/North.scala:3
    //           ^ d: src/main/scala/tureens/North.scala:4
  }
  def wild(): Int = {
    import tureens.north._
    new Tureen().ladle()
    //  ^ d: !jump
    //           ^ d: !jump
  }
}

object Pantry {
  import tureens.south.Tureen
  val kept = new Tureen
  //             ^ d: src/main/scala/tureens/South.scala:3
}

object Scullery {
  import tureens.north.Tureen
  val kept = new Tureen
  //             ^ d: src/main/scala/tureens/North.scala:3
}
