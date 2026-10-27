class Scoop {
  int heap() => 1;
  int call() => 0;
}

class Spoon {
  int heap() => 2;
}

class Caddy {
  final Scoop scoop;
  final Spoon spoon;
  final Spoon ladle = Spoon();
  final tongs = Scoop();
  Scoop pick() => scoop;
  Caddy(this.scoop, this.spoon);
}

class Shelf {
  final Caddy caddy;
  Shelf(this.caddy);
}

int measure(Shelf shelf, Shelf? spare) {
  final heaped = shelf.caddy.scoop.heap();
  //                               ^ d: lib/caddy.dart:2
  //                               status: via Scoop
  final flat = shelf.caddy.spoon.heap();
  //                             ^ d: lib/caddy.dart:7
  final spared = spare?.caddy.scoop.heap() ?? 0;
  //                                ^ d: lib/caddy.dart:2
  final ladled = shelf.caddy.ladle.heap();
  //                               ^ d: lib/caddy.dart:7
  final tonged = shelf.caddy.tongs.heap();
  //                               ^ d: lib/caddy.dart:2
  final torn = shelf.caddy.pick.call();
  //                            ^ d: !jump
  return heaped + flat + spared + ladled + tonged + torn + 'tin: ${shelf.caddy.spoon.heap()}'.length;
  //                                                                                 ^ d: lib/caddy.dart:7
}
