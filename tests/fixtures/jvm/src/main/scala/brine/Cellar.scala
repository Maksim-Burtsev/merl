package brine

import vats.{Barrel}
import Jar.{Crock}

// Both brace imports are relative, to the package and to an object (#1499 review).
class Cellar {
  def run(): Unit = {
    val b: Barrel = new Barrel()
    b.bung()
    //^ d: src/main/scala/brine/vats/Barrel.scala:4
    val c: Crock = new Crock()
    c.pickle()
    //^ d: src/main/scala/brine/Jar.scala:5
  }
}
