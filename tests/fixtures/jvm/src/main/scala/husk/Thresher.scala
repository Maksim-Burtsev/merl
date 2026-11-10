package husk

object Thresher {
  def outer(): Int = { def winnow(): Int = 2; winnow() }
  //                                          ^ d: src/main/scala/husk/Thresher.scala:4

  def sift(): Int = {
    def winnow(): Int = 3
    winnow()
//  ^ d: src/main/scala/husk/Thresher.scala:8
  }

  def flail(): Int = {
    val n = 1
    def winnow(k: Int): Int =
      k + 1
    winnow(n)
//  ^ d: src/main/scala/husk/Thresher.scala:15
  }
}

object Barn {
  def winnow(): Int = 1

  def sift(): Int = {
    def winnow(): Int = 4
    winnow()
//  ^ d: src/main/scala/husk/Thresher.scala:26
  }

  def plain(): Int = winnow()
  //                 ^ d: src/main/scala/husk/Thresher.scala:23
}

object Granary {
  def winnow(): Int = 5

  def fact(n: Int): Int = {
    def winnow(k: Int): Int =
      if (k == 0) 1 else winnow(k - 1)
    //                   ^ d: src/main/scala/husk/Thresher.scala:39
    winnow(n)
//  ^ d: src/main/scala/husk/Thresher.scala:39
  }

  def after(): Int = {
    val n = 2
    if (n > 1) {
      def winnow(): Int = 6
      winnow()
//    ^ d: src/main/scala/husk/Thresher.scala:49
    }
    winnow()
//  ^ d: src/main/scala/husk/Thresher.scala:36
  }
}
