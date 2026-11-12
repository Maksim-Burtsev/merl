package rookery

import Hound.Setter

// A local `val Setter` hides the imported case (#1448).
object Shadowed {
  def pick(): Gem = {
    val Setter = Gem.Jewel
    Setter
 // ^ d: src/main/scala/rookery/Shadowed.scala:8
  }
}
