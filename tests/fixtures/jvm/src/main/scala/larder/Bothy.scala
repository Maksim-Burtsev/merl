package larder

class Bothy {
  def thaw(): Unit = ()

  class Loft {
    def thaw(): Unit = ()
  }
}

trait Annex { me: Bothy =>
  def warm(): Unit = {
    me.thaw()
//     ^ d: src/main/scala/larder/Bothy.scala:4
    me.warm()
//     ^ d: src/main/scala/larder/Bothy.scala:12
  }
}

trait Lean { me: Bothy =>
  def thaw(): Unit = ()
  def warm(): Unit = {
    me.thaw()
//     ^ d: src/main/scala/larder/Bothy.scala:21
  }
}
