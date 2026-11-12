package saucepan.use

import saucepan.{Whisk, Saucepan}
import saucepan.Drying.*

def dryAll(pot: Saucepan, whisk: Whisk): Unit =
  pot.dry()
  //  ^ d: src/main/scala/saucepan/Drying.scala:10
  whisk.dry()
  //    ^ d: src/main/scala/saucepan/Drying.scala:7
