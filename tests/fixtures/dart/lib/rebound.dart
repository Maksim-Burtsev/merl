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
