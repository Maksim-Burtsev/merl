package alchemy

import alchemy.lab.{Retort => Crucible}

object Renamed {
  def run(): Int = {
    val a = new Crucible()
    //          ^ d: src/main/scala/alchemy/Lab.scala:3
    a.distil()
    //^ d: src/main/scala/alchemy/Lab.scala:4
    new Crucible().distil()
    //  ^ d: src/main/scala/alchemy/Lab.scala:3
    //             ^ d: src/main/scala/alchemy/Lab.scala:4
  }

  def odd(): Int = {
    val b = new Crucible() { override def distil(): Int = 9 }
    b.distil()
    //^ d: picker src/main/scala/alchemy/Attic.scala:4, src/main/scala/alchemy/Decoy.scala:4, src/main/scala/alchemy/Lab.scala:4
  }
}
