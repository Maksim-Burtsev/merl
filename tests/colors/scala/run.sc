//> using scala 3.5.2
//> using dep com.lihaoyi::os-lib:0.11.3

val files = os.list(os.pwd).filter(_.ext == "scala")
files.foreach(f => println(f.last))
