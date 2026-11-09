package husk

class Bran {
  def sift = 1
}

object Mill {
  def grindBefore = 1
  /* Scala comments nest: /* this one */ closes only the inner one
  class Bran {
    def sift = 0
  }
  def grindBefore = 0
  def grindAfter = 0
  val branOnly = 0
  */
  def grindAfter = 1

  def run(b: Bran): Unit = {
    new Bran()
//      ^ d: src/main/scala/husk/Bran.scala:3
    b.sift
//    ^ d: src/main/scala/husk/Bran.scala:4
    grindBefore
//  ^ d: src/main/scala/husk/Bran.scala:8
    grindAfter
//  ^ d: src/main/scala/husk/Bran.scala:17
    println(branOnly)
//          ^ d: none
  }
}

object Kiln {
  def bake: Int = {
    val loaves = 1
    /* Scala comments nest: /* this one */ closes only the inner one
    val loaves = 0
    */
    loaves
//  ^ d: src/main/scala/husk/Bran.scala:35
  }
}
