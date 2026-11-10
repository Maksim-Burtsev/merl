package husk

object Thresher {
  def outer(): Int = { def winnow(): Int = 2; winnow() }
  //                                          ^ d: src/main/scala/husk/Chaff.scala:4

  def sift(): Int = {
    def winnow(): Int = 3
    winnow()
//  ^ d: src/main/scala/husk/Chaff.scala:8
  }

  def flail(): Int = {
    val n = 1
    def winnow(k: Int): Int =
      k + 1
    winnow(n)
//  ^ d: src/main/scala/husk/Chaff.scala:15
  }
}

object Barn {
  def winnow(): Int = 1

  def sift(): Int = {
    def winnow(): Int = 4
    winnow()
//  ^ d: src/main/scala/husk/Chaff.scala:26
  }

  def plain(): Int = winnow()
  //                 ^ d: src/main/scala/husk/Chaff.scala:23
}
