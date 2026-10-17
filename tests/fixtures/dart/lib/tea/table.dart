import 'kettle.dart';
import 'teapot.dart';
import 'stale/kettle.dart' as stale;

// #931: what Dart's own scoping makes hard for `d`.
void serve(Teapot pot) {
  final kettle = Kettle.cold();
  //             ^ d: lib/tea/kettle.dart:6
  //                    ^ d: lib/tea/kettle.dart:8
  final cold = stale.Kettle.cold();
  //           ^ d: lib/tea/table.dart:3
  //                 ^ d: lib/tea/stale/kettle.dart:1
  //                        ^ d: lib/tea/stale/kettle.dart:2
  kettle.pour(1);
  //     ^ d: lib/tea/kettle.dart:17
  pot.pour(1);
  //  ^ d: lib/tea/teapot.dart:6
  cold.pour(2);
  //   ^ d: lib/tea/stale/kettle.dart:4
  Teapot.cold().decant();
  //     ^ d: lib/tea/teapot.dart:4
  //            ^ d: lib/tea/teapot.dart:8
  Kettle(1)..pour(2)..cups = 3;
  //^ d: picker lib/tea/kettle.dart:6, lib/tea/kettle.dart:7
  //          ^ d: lib/tea/kettle.dart:17
  //                  ^ d: lib/tea/kettle.dart:15
  print(steep(1) + stale.steep(2));
  //    ^ d: lib/tea/kettle_parts.dart:3
  //                     ^ d: lib/tea/stale/kettle.dart:7
}

// A parameter named like an import prefix hides it.
int pourAll(Teapot stale, int steep) {
  return stale.pour(steep);
  //     ^ d: lib/tea/table.dart:33
  //           ^ d: lib/tea/teapot.dart:6
  //                ^ d: lib/tea/table.dart:33
}

class Samovar extends Kettle {
  Samovar(super.cups) : super.boiled();
  //                          ^ d: lib/tea/kettle.dart:9

  @override
  int pour(int cups) => super.pour(cups) + 1;
  //                          ^ d: lib/tea/kettle.dart:17

  int brew() {
    int steep(int n) => n;
    final (cups, leaves) = (steep(1), 2);
    //                      ^ d: lib/tea/table.dart:49
    return cups + leaves + this.cups;
    //     ^ d: lib/tea/table.dart:50
    //            ^ d: lib/tea/table.dart:50
    //                          ^ d: lib/tea/kettle.dart:15
  }

  int brewed(Object o) => switch (o) {
    Kettle(:final cups) => cups,
    //                     ^ d: lib/tea/table.dart:59
    _ => 0,
  };
}

// What a raw string, a nested comment and a doc comment hold declares nothing.
const recipe = r'''
class Banshee {
  Banshee.boo();
''';

/* outer
/* inner */
class Wraith {}
*/

/// class Specter {}
void spook() => print([Banshee.boo, Wraith, Specter]);
//                     ^ d: none
//                             ^ d: none
//                                  ^ d: none
//                                          ^ d: none

// A named argument's label is no local of the call's own function.
int steepFor({int minutes = 1}) => minutes;
int timed(int minutes) => steepFor(minutes: minutes);
//                                 ^ d: none
//                                          ^ d: lib/tea/table.dart:85
int timedOver(int minutes) => steepFor(
  minutes: minutes,
//^ d: none
//         ^ d: lib/tea/table.dart:88
);

// A local used as a map key, or between a `?` and its `:`, is still the local.
Map<int, int> cupsBy(int cups, bool full) => {cups: full ? cups: 0};
//                                            ^ d: lib/tea/table.dart:95
//                                                         ^ d: lib/tea/table.dart:95

// Inside a class with a superclass, an inherited member may be the namesake.
class Brewer extends Samovar {
  int again() => steep(1);
  //             ^ d: picker lib/tea/kettle_parts.dart:3, lib/tea/table.dart:49
}
