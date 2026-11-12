package alchemy

class Alembic[T] {
  def condense(): Int = 4
}

class Pelican {
  def condense(): Int = 5
}

object Still {
  def run(): Int = {
    val a = new Alembic[Int]()
    a.condense()
    //^ d: src/main/scala/alchemy/Still.scala:4
  }
}
