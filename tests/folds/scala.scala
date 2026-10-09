// Every construct `f` folds in Scala, and the cases that once broke it.
package a  // f: none

import a.{  // f: 4-7
  B,  // f: 4-7
  C  // f: 4-7
}  // f: 4-7
import d.E  // f: none

@deprecated("x", "1")  // f: 10-14
final case class Point(  // f: 10-14
    x: Int,  // f: 10-14
    y: Int  // f: 10-14
) extends Shape  // f: 10-14

sealed trait Shape {  // f: 16-18
  def area: Double  // f: 16-18
}  // f: 16-18

object Geometry extends App {  // f: 20-76
  private[a] val origin =  // f: 21-23
    Point(0, 0)  // f: 20-76

  @tailrec  // f: 24-28
  def gcd(a: Int, b: Int): Int =  // f: 24-28
    if (b == 0) a  // f: 24-28
    else gcd(b, a % b)  // f: 24-28

  def describe(s: Shape): String = s match {  // f: 29-32
    case Point(x, y) => s"point ${x + "}"}"  // f: 29-32
    case _ => "shape"  // f: 29-32
  }  // f: 29-32

  def run(xs: Seq[Int]): Unit = {  // f: 34-67
    for (x <- xs)  // f: 35-37
      println(x)  // f: 34-67

    val ys = for {  // f: 38-41
      x <- xs  // f: 34-67
      if x > 0  // f: 34-67
    } yield x * 2  // f: 34-67

    while (ys.isEmpty)  // f: 43-44
      wait()  // f: 34-67
    try {  // f: 45-51
      go()  // f: 34-67
    } catch {  // f: 34-67
      case e: Exception => log(e)  // f: 34-67
    } finally {  // f: 34-67
      done()  // f: 34-67
    }  // f: 34-67
    xs.map { x =>  // f: 52-54
      x + 1  // f: 34-67
    }  // f: 34-67
    xs foreach { x =>  // f: 34-67
      println(x)  // f: 34-67
    }  // f: 34-67
    xs.collect { case x if x > 0 =>  // f: 34-67
      x  // f: 34-67
    }  // f: 34-67
    test("brace '}' in a string") {  // f: 61-66
      val c = '{'  // f: 34-67
      val q = """  // f: 63-65
        }}}
      """
    }
  }  // f: 34-67

  lazy val table: Map[String, Int] = Map(  // f: 69-72
    "a" -> 1,  // f: 20-76
    "b" -> 2  // f: 20-76
  )  // f: 20-76
  /* a /* nested */ comment
     with a brace { */
  var count = 0  // f: 20-76
}  // f: 20-76

def sign(x: Int): Int =  // f: 78-83
  if x > 0 then  // f: 78-83
    1  // f: 78-83
  else  // f: 78-83
    -1  // f: 78-83

def twice(x: Int): Int =  // f: 84-86
  val y = x  // f: 84-86
  y * 2  // f: 84-86
