class Brazier {
  factory Brazier.forge() => Brazier._();
  Brazier._();
  static Brazier make() => Brazier._();
  void light() {}
}

class Grate {
  static Grate make() => Grate();
  void light() {}
}

void stoke() {
  Brazier.forge().light();
  //              ^ d: picker lib/brazier.dart:5, lib/brazier.dart:10; want lib/brazier.dart:5 (#1489)
  Brazier.make().light();
  //             ^ d: lib/brazier.dart:5
  Brazier.forge();
  //      ^ d: lib/brazier.dart:2
}
