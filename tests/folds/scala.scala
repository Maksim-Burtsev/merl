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

def twice(x: Int): Int =  // f: 84-87
  val y = x  // f: 84-87
  y * 2  // f: 84-87

object Main:  // f: 88-98
  def run(x: Int): Int =  // f: 89-92
    val y = x  // f: 89-92
    y * 2  // f: 89-92

  def other(): Unit =  // f: 93-95
    println(x)  // f: 93-95
  end other  // f: 93-95
  val z = 1  // f: 88-98
// a comment at the margin  // f: 88-98

class Box(x: Int) extends Shape:  // f: 99-101
  def area: Double = x  // f: 99-101
end Box  // f: 99-101

object Outer:  // f: 103-106
  def f =  // f: 104-105
    1  // f: 104-105
end Outer  // f: 103-106

def kind(x: Int): String = x match  // f: 108-111
  case 1 => "one"  // f: 108-111
  case _ => "many"  // f: 108-111

def loop(xs: Seq[Int]): Unit =  // f: 112-120
  while xs.isEmpty do  // f: 113-115
    wait()  // f: 112-120
  end while  // f: 112-120
  xs.head match  // f: 116-119
    case 1 => go()  // f: 112-120
    case _ => stop()  // f: 112-120
  end match  // f: 112-120
end loop  // f: 112-120

def plus(x: Int): Int =  // f: 122-125
  val y = x  // f: 122-125
  y + 1  // f: 122-125
end plus  // f: 122-125

object Text:  // f: 127-132
  val s = """  // f: 128-130
a
""".stripMargin  // f: 127-132
  val t = 1  // f: 127-132

import a.b.*  // f: none
import c.*  // f: none

object Wild:  // f: 136-138
  val w = 1  // f: 136-138

class Phase:  // f: 139-161
  def loop(t: Int): Int = t match  // f: 140-144
    case 1 =>  // f: 140-144
      2  // f: 140-144
    case _ =>  // f: 140-144

  def make = new Runnable:  // f: 145-149
    import a.*  // f: 145-149

    def run(): Unit = ()  // f: 145-149

  def same(x: Int) =  // f: 150-154
    x match  // f: 151-154
    case 1 => 2  // f: 150-154
    case _ => 3  // f: 150-154

  def dotted(x: Int) = x  // f: 155-160
    || x.match  // f: 156-160
      case 1 => true  // f: 155-160
      case _ => false  // f: 155-160

  val last = 1  // f: 155-160

trait Last:  // f: 162-164
  def a: Int  // f: 162-164

class Wrapped[K, V]  // f: 165-250
    (capacity: Int)  // f: 165-250
extends Base[K, V](capacity):  // f: 165-250
  def size = 0  // f: 165-250

  def drain(): Unit =  // f: 170-178
    try  // f: 171-177
      val run = make()  // f: 170-178
      run.go()  // f: 170-178
    catch  // f: 170-178
      case e: Exception =>  // f: 170-178
        log(e)  // f: 170-178
    done()  // f: 170-178

  def pairs(xs: Seq[Int]): Unit =  // f: 179-185
    for  // f: 180-185
      x <- xs  // f: 179-185
      y <- xs  // f: 179-185
    do  // f: 183-185
      println(x + y)  // f: 179-185

  def spin(i: Int): Unit =  // f: 186-190
    while  // f: 187-189
      step(i)  // f: 186-190
    do ()  // f: 186-190

  def both(xs: Seq[Int]) =  // f: 191-194
    for x <- xs; y <- xs yield  // f: 192-194
      x + y  // f: 191-194

  def check(a: Boolean, b: Boolean): Boolean =  // f: 195-200
    val ok =  // f: 196-199
         a  // f: 195-200
      || b  // f: 195-200
    ok  // f: 195-200

  def find(e: Int, dense: Boolean): Int =  // f: 201-207
    if dense then  // f: 201-207
      while e < 0 do  // f: 203-205
        step(e)  // f: 201-207
    else  // f: 201-207
      0  // f: 201-207

  def walk(t: Tree): Unit = t match  // f: 208-213
    case r: Ref => r.tpe match  // f: 209-212
      case Term(p) => go(p)  // f: 208-213
      case _ =>  // f: 208-213
    case _ =>  // f: 208-213

  def guard(p: Int): String = p match  // f: 214-219
  case 1 => "one"  // f: 214-219
  case _  // f: 214-219
  if p > 9 => "big"  // f: 214-219
  case _ => "other"  // f: 214-219

  def lift(xs: Seq[Int]) = xs.map(n =>  // f: 220-224
    val m =  // f: 221-223
      n * 2  // f: 220-224
    m + 1  // f: 220-224
  )  // f: 220-224

  def drop(plan: Plan) =  // f: 226-231
    val Let(top, _) = plan: @unchecked  // f: 226-231

    def inner(s: Int) =  // f: 229-231
      s + 1  // f: 229-231

  def sym(x: Int): Symbol { type Name = String } =  // f: 232-235
    val s = make(x)  // f: 232-235
    s  // f: 232-235

  def short(sym: Symbol) =  // f: 236-238
    sym == defn.Boolean_&& || sym == defn.Boolean_||  // f: 236-238

  def traverse(t: Tree): Unit = t match  // f: 239-242
    case a: A => go(a); go(a)  // f: 239-242
    case _ => ()  // f: 239-242
  end traverse  // f: 239-242

  def used(s: Sym): Boolean =  // f: 244-250
       s.isA  // f: 244-250
    || s.tpe.match  // f: 246-250
         case T => true  // f: 244-250
         case _ => false  // f: 244-250
    || s.isB  // f: 244-250
  val last = 1  // f: 244-250
