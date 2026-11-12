package saucepan.use

import saucepan.{Ladle, Tongs}
import saucepan.Sealing.*

def claspAll(ladle: Ladle[Int], tongs: Tongs): Unit =
  ladle.clasp()
  //    ^ d: picker src/main/scala/saucepan/Sealing.scala:9, src/main/scala/saucepan/Sealing.scala:12; want src/main/scala/saucepan/Sealing.scala:9 (#1499)
  tongs.clasp()
  //    ^ d: src/main/scala/saucepan/Sealing.scala:12
  ladle.swab()
  //    ^ d: !jump
  tongs.swab()
  //    ^ d: src/main/scala/saucepan/Sealing.scala:18

def claspMade(): Unit =
  val ladle = new Ladle[Int]()
  ladle.clasp()
  //    ^ d: src/main/scala/saucepan/Sealing.scala:9
