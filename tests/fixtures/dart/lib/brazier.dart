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
  //              ^ d: lib/brazier.dart:5
  Brazier.make().light();
  //             ^ d: lib/brazier.dart:5
  Brazier.forge();
  //      ^ d: lib/brazier.dart:2
}

void rake() {
  Grate.make().light();
  //           ^ d: lib/brazier.dart:10
}

class Hearth {
  Hearth();
  static Grate forge() => Grate();
  void light() {}
}

void bank() {
  Hearth.forge().light();
  //             ^ d: lib/brazier.dart:10
}
