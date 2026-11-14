package saucepan.use

def stirOutside(ladle: java.io.File): Unit =
  ladle.stir()
  //    ^ d: !jump

def stirUntyped(): Unit =
  val ladle = mystery()
  ladle.stir()
  //    ^ d: !jump

def stirLambda(xs: List[Int]): Unit =
  xs.foreach { ladle => ladle.stir() }
  //                          ^ d: !jump

def stirTyped(ladle: saucepan.Tongs): Unit =
  ladle.stir()
  //    ^ d: !jump

def stirOther(spoon: java.io.File): Unit =
  spoon.stir()
  //    ^ d: !jump

class Stirrer(ladle: java.io.File) {
  def go(): Unit = ladle.stir()
  //                     ^ d: !jump
  def again(): Unit = {
    ladle.stir()
    //    ^ d: !jump
  }
}

class Whisker {
  private val ladle = mystery()
  def go(): Unit = {
    ladle.stir()
    //    ^ d: !jump
  }
}

def stirPackage(): Unit =
  ladle.stir()
  //    ^ d: src/main/kotlin/ladle/Stir.kt:3

case class Mixer(paddle: java.io.File) {
  def go(): Unit = paddle.stir()
  //                      ^ d: !jump
}

class Beater(
    n: Int,
    ladle: java.io.File
) {
  def go(): Unit = ladle.stir()
  //                     ^ d: !jump
}

class Folder(n: Int)(ladle: java.io.File) {
  def go(): Unit = ladle.stir()
  //                     ^ d: !jump
}

class Churner[A](ladle: A) {
  def go(): Unit = ladle.stir()
  //                     ^ d: !jump
}

class Whipper(ladle: java.io.File):
  def go(): Unit =
    ladle.stir()
    //    ^ d: !jump

trait Swirler(ladle: java.io.File) {
  def go(): Unit = ladle.stir()
  //                     ^ d: !jump
}

class Spooner(spoon: java.io.File) {
  def go(): Unit = ladle.stir()
  //                     ^ d: src/main/kotlin/ladle/Stir.kt:3
}

class Outer(ladle: java.io.File) {
  def go(): Unit = ()
}

class Later {
  def go(): Unit = ladle.stir()
  //                     ^ d: src/main/kotlin/ladle/Stir.kt:3
}

class Pot(ladle: java.io.File) {
  class Lid {
    def go(): Unit = ladle.stir()
    //                     ^ d: !jump
  }
}

class Kettle(n: Int)(using ladle: java.io.File) {
  def go(): Unit = ladle.stir()
  //                     ^ d: !jump
}
