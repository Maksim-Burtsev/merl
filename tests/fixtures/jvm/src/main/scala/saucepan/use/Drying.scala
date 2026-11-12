package saucepan.use

import saucepan.{Ladle, Saucepan}
import saucepan.Drying.*

def dryAll(pot: Saucepan, ladle: Ladle): Unit =
  pot.dry()
  //  ^ d: src/main/scala/saucepan/Drying.scala:10
  ladle.dry()
  //    ^ d: src/main/scala/saucepan/Drying.scala:7
