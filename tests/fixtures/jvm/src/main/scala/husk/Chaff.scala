package husk

class Hull {
  def shed(): Int = 1
}

class Pod {
  def shed(): Int = 2
}

class Winnower(h: Hull, val p: Pod) {
  def run(): Int = {
    h.shed()
//    ^ d: src/main/scala/husk/Chaff.scala:4
    p.shed()
//    ^ d: src/main/scala/husk/Chaff.scala:8
  }

  def again(h: Pod): Int = {
    h.shed()
//    ^ d: src/main/scala/husk/Chaff.scala:8
  }
}
