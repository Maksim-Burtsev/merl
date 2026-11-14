package quoted

class Stylus {
  def blot(): String = "stylus"
}

class Nib {
  def blot(): String = "nib"
}

object Quoted {
  def a(s: Stylus): String = s"s.blot() ${s.blot()}" + "${s.blot()}"
  //                             ^ d: picker src/main/scala/quoted/Quoted.scala:4, src/main/scala/quoted/Quoted.scala:8, …
  //                                        ^ d: src/main/scala/quoted/Quoted.scala:4
  //                                                        ^ d: picker src/main/scala/quoted/Quoted.scala:4, src/main/scala/quoted/Quoted.scala:8, …
  def b(s: Stylus): String = """
    s.blot()
  //  ^ d: picker src/main/scala/quoted/Quoted.scala:4, src/main/scala/quoted/Quoted.scala:8, …
  """
}
