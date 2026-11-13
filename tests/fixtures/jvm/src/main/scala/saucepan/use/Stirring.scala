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
  //                     ^ d: src/main/kotlin/ladle/Stir.kt:3; want !jump (#1520)
  def again(): Unit = {
    ladle.stir()
    //    ^ d: src/main/kotlin/ladle/Stir.kt:3; want !jump (#1520)
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
