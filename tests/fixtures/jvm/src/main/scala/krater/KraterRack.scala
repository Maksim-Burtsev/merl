package krater

class KraterRack[Krater](val held: Krater) {
//               ^ d: src/main/scala/krater/KraterRack.scala:3
//                                 ^ d: src/main/scala/krater/KraterRack.scala:3
  def take(x: Krater): Krater = x
  //          ^ d: src/main/scala/krater/KraterRack.scala:3
  //                   ^ d: src/main/scala/krater/KraterRack.scala:3
}

object KraterFns {
  def one[Krater](x: Krater): Krater = x
  //      ^ d: src/main/scala/krater/KraterRack.scala:12
  //                 ^ d: src/main/scala/krater/KraterRack.scala:12
  //                          ^ d: src/main/scala/krater/KraterRack.scala:12
  def plain(x: Krater): Krater = x
  //           ^ d: src/main/scala/krater/Krater.scala:3
  //                    ^ d: src/main/scala/krater/Krater.scala:3
}

class KraterBound[T <: Krater](val t: T)
//                ^ d: src/main/scala/krater/KraterRack.scala:21
//                                    ^ d: src/main/scala/krater/KraterRack.scala:21
//                     ^ d: src/main/scala/krater/Krater.scala:3

trait KraterHolder[Krater] {
//                 ^ d: src/main/scala/krater/KraterRack.scala:26
  def get(): Krater
  //         ^ d: src/main/scala/krater/KraterRack.scala:26
}

class KraterOuter[Krater] {
//                ^ d: src/main/scala/krater/KraterRack.scala:32
  def inner[Krater](x: Krater): Krater = x
  //        ^ d: src/main/scala/krater/KraterRack.scala:34
  //                   ^ d: src/main/scala/krater/KraterRack.scala:34
  //                            ^ d: src/main/scala/krater/KraterRack.scala:34
  def other(x: Krater): Krater = x
  //           ^ d: src/main/scala/krater/KraterRack.scala:32
  //                    ^ d: src/main/scala/krater/KraterRack.scala:32
}

object KraterGivens {
  given Ordering[Krater] with
  //             ^ d: src/main/scala/krater/Krater.scala:3
    def compare(a: Krater, b: Krater): Int = 0
    //             ^ d: src/main/scala/krater/Krater.scala:3
}

class KraterWrapped[
    Krater
//  ^ d: src/main/scala/krater/KraterRack.scala:51
](val held: Krater) {
//          ^ d: src/main/scala/krater/KraterRack.scala:51
  def take(x: Krater): Krater = x
  //          ^ d: src/main/scala/krater/KraterRack.scala:51
}

object KraterWrappedFns {
  def wrapped[Krater](
      x: Krater,
      // ^ d: src/main/scala/krater/KraterRack.scala:60
      y: Int
  ): Krater = x
  // ^ d: src/main/scala/krater/KraterRack.scala:60
  def plain(
      x: Krater
      // ^ d: src/main/scala/krater/Krater.scala:3
  ): Krater = x
  // ^ d: src/main/scala/krater/Krater.scala:3
}
