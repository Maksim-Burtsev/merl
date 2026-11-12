package rebound

class Casts(v: Any):
  def cast(): Unit =
    v.asInstanceOf[Cup].sip()
    //                  ^ d: src/main/scala/rebound/Cup.scala:4
    val k = v.asInstanceOf[Mug]
    k.sip()
    //^ d: src/main/scala/rebound/Mug.scala:4
