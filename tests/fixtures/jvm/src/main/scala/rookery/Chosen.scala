package rookery

import Gem.*
import Hound.Setter

// An explicit import wins over a wildcard one (#1448).
object Chosen {
  val hound = Setter
           // ^ d: src/main/scala/rookery/Hound.scala:4
}
