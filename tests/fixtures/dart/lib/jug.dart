import 'kiln.dart' if (dart.library.html) 'kiln.dart';

class Vessel {
  int volume = 0;
}

mixin Lidded {
  bool sealed = false;
}

abstract class Pourer {
  void pourOut();
}

class Jug extends Vessel with Lidded implements Pourer {
  @override
  void pourOut() {}
}

class Bowl {
  int volume = 1;
  bool sealed = true;
  int slots = 2;
  void pourOut() {}
}

class _Tray {
  int slots = 0;
}

abstract class Spout implements Pourer {}

// A member the receiver's class inherits: from its base, its mixin, an interface (#1085).
void serve(Jug jug, Spout spout, _Tray tray) {
  print([jug.volume, jug.sealed, spout.pourOut]);
  //         ^ d: lib/jug.dart:4
  //                     ^ d: lib/jug.dart:8
  //                                   ^ d: lib/jug.dart:12
  // A private class as the written type (#1085).
  print(tray.slots);
  //         ^ d: lib/jug.dart:28
}

// A conditional import no longer turns off the class-versus-constructor filter (#1085).
void bake(Kiln kiln) {}
//        ^ d: lib/kiln.dart:1
