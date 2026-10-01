package courier.app

object Route:
  def legs(stops: List[String]): Int = stops.size - 1

case class Stop(code: String)

def plan(stop: Stop): Int = Route.legs(List(stop.code))
