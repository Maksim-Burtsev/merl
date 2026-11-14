package alchemy

import alchemy.{lab => l}

// A local `val l` or a parameter `l` hides the renamed package (#1466).
object Shadowing {
  def local(): Int = {
    val l = new alchemy.decoy.Crucible()
    l.distil()
    //^ d: picker src/main/scala/alchemy/Attic.scala:4, src/main/scala/alchemy/Decoy.scala:4, src/main/scala/alchemy/Lab.scala:4
  }

  def param(l: alchemy.decoy.Crucible): Int = l.distil()
  //                                            ^ d: picker src/main/scala/alchemy/Attic.scala:4, src/main/scala/alchemy/Decoy.scala:4, src/main/scala/alchemy/Lab.scala:4
}
