package rookery

import Gem.*

object Board {
  def advance(piece: Gem): Int = piece match {
    case Setter => 1
      // ^ d: src/main/scala/rookery/Gem.scala:4
    case Jewel => 0
  }
}
