object Rich {
  implicit class RichString(val s: String) {
    def holler(): String = s.toUpperCase
  }
}
