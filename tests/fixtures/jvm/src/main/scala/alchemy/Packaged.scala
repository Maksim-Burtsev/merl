package alchemy

import alchemy.{lab => l}

object Packaged {
  def run(): Int = new l.Retort().distil()
  //                     ^ d: src/main/scala/alchemy/Lab.scala:3
  //                              ^ d: src/main/scala/alchemy/Lab.scala:4
}
