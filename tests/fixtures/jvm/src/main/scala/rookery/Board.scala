package rookery

import Gem.*

object Board {
  def advance(piece: Gem): Int = piece match {
    case Setter => 1
      // ^ d: picker src/main/scala/rookery/Gem.scala:4, src/main/scala/rookery/Hound.scala:4; want src/main/scala/rookery/Gem.scala:4 (#1448)
    case Jewel => 0
  }
}
