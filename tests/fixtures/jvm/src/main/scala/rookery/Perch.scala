package rookery

import Hound.*

// A case two enums of the project share, written without its enum (#1448).
object Perch {
  def perch(bird: Hound): Int = bird match {
    case Setter => 1
      // ^ d: picker src/main/scala/rookery/Gem.scala:4, src/main/scala/rookery/Hound.scala:4; want src/main/scala/rookery/Hound.scala:4 (#1448)
    case Beagle => 0
  }

  def advance(piece: Gem): Int = piece match {
    case Gem.Setter => 1
          // ^ d: src/main/scala/rookery/Gem.scala:4
    case _ => 0
  }

  val plain: Hound = Setter
                  // ^ d: picker src/main/scala/rookery/Gem.scala:4, src/main/scala/rookery/Hound.scala:4; want src/main/scala/rookery/Hound.scala:4 (#1448)
}
