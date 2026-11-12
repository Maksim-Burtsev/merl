package alchemy

import alchemy.lab.{Retort as Crucible}

object Three {
  def run(): Int = {
    val a = Crucible()
    //      ^ d: src/main/scala/alchemy/Lab.scala:3
    a.distil()
    //^ d: src/main/scala/alchemy/Lab.scala:4
  }
}
