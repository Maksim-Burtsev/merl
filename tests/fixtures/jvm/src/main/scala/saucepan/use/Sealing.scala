package saucepan.use

import saucepan.{Ladle, Tongs}
import saucepan.Sealing.*

def claspAll(ladle: Ladle[Int], tongs: Tongs): Unit =
  ladle.clasp()
  //    ^ d: src/main/scala/saucepan/Sealing.scala:9
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

def otherShapes(ladle: Ladle[Int], tongs: Tongs): Unit =
  ladle.stir()
  //    ^ d: src/main/scala/saucepan/Sealing.scala:3
  ladle.rub()
  //    ^ d: !jump
  ladle.wipe()
  //    ^ d: !jump
  tongs.rub()
  //    ^ d: src/main/scala/saucepan/Sealing.scala:24
