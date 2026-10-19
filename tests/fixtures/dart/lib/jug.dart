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

// A field as the receiver, typed in the class body (#1085).
class Pantry {
  Pantry(this._bowl);

  final Bowl _bowl;
  Jug? spare;
  int volume = 3;

  void stock() {
    print(_bowl.volume);
    //          ^ d: lib/jug.dart:21
    print(spare?.volume);
    //           ^ d: lib/jug.dart:4
  }

  void shadow(Jug _bowl) {
    print(_bowl.volume);
    //          ^ d: lib/jug.dart:4
  }
}

// A getter and its setter: an assignment lands on the setter, a read on the getter (#1085).
class Lid {
  String get tone => '';
  set tone(String value) {}
}

void paint(Lid lid) {
  lid.tone = lid.tone;
  //  ^ d: lib/jug.dart:72
  //             ^ d: lib/jug.dart:71
  lid.tone += 'x';
  //  ^ d: picker lib/jug.dart:71, lib/jug.dart:72
}

// `++` reads and writes: both accessors; a project class with its own constructor: the class and the constructor.
class Pot {
  final int brim;
  Pot(this.brim);
}

void brew(Lid lid) {
  ++lid.tone;
  //    ^ d: picker lib/jug.dart:71, lib/jug.dart:72
  Pot(1);
  //^ d: picker lib/jug.dart:84, lib/jug.dart:86
}
