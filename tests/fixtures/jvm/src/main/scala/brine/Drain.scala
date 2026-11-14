package brine

import java.util.{List => JList, _}

// The `_` selector is a wildcard, never an import named `_` (#1499 review).
def drain(ys: Seq[Jar.Crock]): Unit =
  ys.foreach(_.pickle())
  //           ^ d: src/main/scala/brine/Jar.scala:5
