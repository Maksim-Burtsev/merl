package alchemy

import alchemy.{lab => l}

object Packaged {
  def run(): Int = new l.Retort().distil()
  //                     ^ d: picker src/main/scala/alchemy/Attic.scala:3, src/main/scala/alchemy/Lab.scala:3; want src/main/scala/alchemy/Lab.scala:3 (#1466)
  //                              ^ d: picker src/main/scala/alchemy/Attic.scala:4, src/main/scala/alchemy/Decoy.scala:4, src/main/scala/alchemy/Lab.scala:4; want src/main/scala/alchemy/Lab.scala:4 (#1466)
}
