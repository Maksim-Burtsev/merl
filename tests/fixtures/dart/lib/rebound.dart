class Vase {
  void slosh() {}
}

class Ewer extends Vase {
  @override
  void slosh() {}
}

class Flagon {
  void slosh() {}
}

void sameLine(Flagon x, List<Ewer> es) {
  es.forEach((Ewer x) => x.slosh()); x.slosh();
  //                       ^ d: lib/rebound.dart:7
  //                                   ^ d: lib/rebound.dart:11
  for (final x in es) x.slosh(); x.slosh();
  //                    ^ d: lib/rebound.dart:7
  //                               ^ d: lib/rebound.dart:11
  print(es.map((x) => 1)); x.slosh();
  //                         ^ d: lib/rebound.dart:11
  if (x.hashCode > 0) { final x = Ewer(); x.slosh(); }
  //                                        ^ d: lib/rebound.dart:7
  if (x.hashCode > 0) { final x = Ewer(); } x.slosh();
  //                                          ^ d: lib/rebound.dart:11
  final q = 'case Ewer x: (x) => {'; x.slosh();
  //                                   ^ d: lib/rebound.dart:11
}

void patterns(Flagon x, Object o) {
  if (o case Ewer x) {
    x.slosh();
    //^ d: lib/rebound.dart:7
  }
  if (o case Ewer x) x.slosh(); x.slosh();
  //                   ^ d: lib/rebound.dart:7
  //                              ^ d: lib/rebound.dart:11
  switch (o) {
    case Ewer x when x.hashCode > 0:
      x.slosh();
      //^ d: picker lib/rebound.dart:2, lib/rebound.dart:7, lib/rebound.dart:11
  }
  final r = switch (o) { Ewer x => x.slosh(), _ => null };
  //                                 ^ d: lib/rebound.dart:7
  final s = switch (o) {
    Ewer x => x.slosh(),
    //          ^ d: lib/rebound.dart:7
    _ => x.slosh(),
    //     ^ d: lib/rebound.dart:11
  };
  x.slosh();
  //^ d: lib/rebound.dart:11
}

void promoted(Vase x) {
  if (x is Ewer) {
    x.slosh();
    //^ d: picker lib/rebound.dart:2, lib/rebound.dart:7, lib/rebound.dart:11
  }
  x.slosh();
  //^ d: lib/rebound.dart:2
  x is Ewer ? x.slosh() : null;
  //            ^ d: picker lib/rebound.dart:2, lib/rebound.dart:7, lib/rebound.dart:11
  if (x is Ewer) {} x.slosh();
  //                  ^ d: lib/rebound.dart:2
  if (x is! Ewer) return;
  x.slosh();
  //^ d: picker lib/rebound.dart:2, lib/rebound.dart:7, lib/rebound.dart:11
}

void typedAt(Vase x, Object o, List<Flagon> fs) {
  if (o case Ewer x) {} for (final x in fs) x.slosh();
  //                                          ^ d: picker lib/rebound.dart:2, lib/rebound.dart:7, lib/rebound.dart:11
  if (o case Ewer x) {} switch (o) { case Flagon x: x.slosh(); }
  //                                                  ^ d: picker lib/rebound.dart:2, lib/rebound.dart:7, lib/rebound.dart:11
  { Ewer x = Ewer(); } for (final x in fs) x.slosh();
  //                                         ^ d: picker lib/rebound.dart:2, lib/rebound.dart:7, lib/rebound.dart:11
}

void notNarrowed(Vase x, bool ok, List<Flagon> fs) {
  if (x is Ewer || ok) {
    x.slosh();
    //^ d: lib/rebound.dart:2
  }
  if (x is Ewer || ok) x.slosh();
  //                     ^ d: lib/rebound.dart:2
  while (!(x is Ewer)) {
    x.slosh();
    //^ d: lib/rebound.dart:2
  }
  group('x is hot', () {
    x.slosh();
    //^ d: lib/rebound.dart:2
  });
  if (x is Ewer) {
    for (final x in fs) {
      x.slosh();
      //^ d: lib/rebound.dart:11
    }
  }
  if (x is! Ewer) {
    print('odd');
  }
  x.slosh();
  //^ d: lib/rebound.dart:2
}

void closedArms(Ewer x, Object o) {
  switch (o) { case Flagon x: break; case _: x.slosh(); }
  //                                           ^ d: lib/rebound.dart:7
  switch (o) { case Flagon x: break; default: x.slosh(); }
  //                                            ^ d: lib/rebound.dart:7
}

class Ledge {
  Ewer x = Ewer();

  void read(Object o) {
    if (o case Flagon x) {} x.slosh();
    //                        ^ d: lib/rebound.dart:7
    final r = switch (o) { Flagon x => 1, _ => x.slosh() };
    //                                           ^ d: lib/rebound.dart:7
  }
}

void group(String s, void Function() f) {}
