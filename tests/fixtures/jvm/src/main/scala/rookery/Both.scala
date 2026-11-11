package rookery

import Gem.*
import Hound.*

// Two enums sharing a case, both imported: Scala calls it ambiguous (#1448).
object Both {
  val either = Setter
            // ^ d: picker src/main/scala/rookery/Gem.scala:4, src/main/scala/rookery/Hound.scala:4
}
