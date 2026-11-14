package rookery

// An import inside an object reaches only that object; merl reads only top-level ones (#1448).
object Nested {
  import Gem.*
  val inside = Setter
            // ^ d: picker src/main/scala/rookery/Gem.scala:4, src/main/scala/rookery/Hound.scala:4
}
