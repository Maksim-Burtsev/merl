package alchemy

import alchemy.decoy.Crucible

object Plain {
  def run(): Int = new Crucible().distil()
  //                   ^ d: src/main/scala/alchemy/Decoy.scala:3
  //                              ^ d: src/main/scala/alchemy/Decoy.scala:4

  def bare(): Int = {
    var c = new Crucible
    c.distil()
    //^ d: src/main/scala/alchemy/Decoy.scala:4
  }
}
