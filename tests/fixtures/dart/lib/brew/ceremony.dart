import 'chawan.dart';
import 'chawan.dart' as c;
import 'old_chawan.dart' as old;
import '../brew.dart' as barrel;

// #1126: the third corner, its answers read from the code.
int sdk(String name) {
  final buffer = StringBuffer();
  buffer.write(name.trim());
  //     ^ d: none
  //                ^ d: none
  return const Duration(minutes: 1).inMinutes + name.padRight(2).length;
  //                                ^ d: none
}

int chain(Whisk w) {
  final made = w.dip(1).dip(2).froth();
  //                    ^ d: lib/brew/chawan.dart:38
  //                           ^ d: lib/brew/chawan.dart:39
  return made.tilt() + makeChawan().heft() + Whisk.bamboo().froth().glaze;
  //          ^ d: lib/brew/chawan.dart:16
  //                                ^ d: lib/brew/chawan.dart:18
  //                                                        ^ d: lib/brew/chawan.dart:39
  //                                                                ^ d: lib/brew/chawan.dart:14
}

int overLines(Whisk w) => w
    .dip(3)
    .froth()
    .tilt();
//   ^ d: lib/brew/chawan.dart:16

Chawan cascade() => Chawan()
  ..glaze = 2
  ..tilt();
//  ^ d: picker lib/brew/chawan.dart:16, lib/brew/chawan.dart:34, lib/brew/old_chawan.dart:2; want lib/brew/chawan.dart:16 (#1072)

int shadow(List<Chawan> chawans) {
  var total = 0;
  for (final one in chawans) {
    //              ^ d: lib/brew/ceremony.dart:38
    total += one.glaze;
    //       ^ d: lib/brew/ceremony.dart:40
  }
  final [one, ...rest] = chawans;
  if (one case Matcha(:final glaze)) {
    total += glaze;
    //       ^ d: lib/brew/ceremony.dart:46
  }
  return total + one.heft() + rest.length;
  //             ^ d: none; want lib/brew/ceremony.dart:45 (#1219)
  //                 ^ d: picker lib/brew/chawan.dart:18, lib/brew/old_chawan.dart:3; want lib/brew/chawan.dart:18 (#1219)
}

int inherited(Matcha m, Chawan ch) {
  return m.heft() +
      //   ^ d: lib/brew/chawan.dart:18
      m.tilt() +
      //^ d: lib/brew/chawan.dart:34
      m.scoop() +
      //^ d: lib/brew/chawan.dart:3
      ch.rinseOut() +
      // ^ d: lib/brew/chawan.dart:21
      m.tiltTwice() +
      //^ d: lib/brew/chawan.dart:51
      'x'.frothed.length;
  //      ^ d: lib/brew/chawan.dart:47
}

int prefixed() {
  final dried = c.Chawan.dry();
  //                     ^ d: lib/brew/chawan.dart:12
  final aged = old.Chawan();
  //               ^ d: lib/brew/old_chawan.dart:1
  final handed = barrel.Whisk();
  //                    ^ d: lib/brew/chawan.dart:37
  return dried.heft() + aged.heft() + handed.froth().heft();
  //           ^ d: lib/brew/chawan.dart:18
  //                         ^ d: lib/brew/old_chawan.dart:3
  //                                                 ^ d: picker lib/brew/chawan.dart:18, lib/brew/old_chawan.dart:3; want lib/brew/chawan.dart:18 (#1221)
}

const ritual = '''
class Ghostly {
  int heft() => 0;
''';

// int heftless() => 0;
String spoken(Chawan ch) => '${ch.tilt()} $ritual ${Ghostly} ${heftless()}';
//                                ^ d: lib/brew/chawan.dart:16
//                                         ^ d: lib/brew/ceremony.dart:83
//                                                  ^ d: none
//                                                             ^ d: none

class Host {
  final Whisk whisk = Whisk();

  Chawan makeChawan() => Matcha();

  int serve(List<Chawan> chawans) {
    int heft(Chawan ch) => ch.tilt();
    return chawans.map((ch) => heft(ch) + whisk.froth().tilt()).length + makeChawan().tilt();
    //                         ^ d: lib/brew/ceremony.dart:101
    //                                          ^ d: lib/brew/chawan.dart:39
    //                                                                   ^ d: lib/brew/ceremony.dart:98
  }

  int graded() => Grade.ceremonial.score + Grade.culinary.doubled() + whisk(2);
  //                    ^ d: lib/brew/chawan.dart:55
  //                               ^ d: lib/brew/chawan.dart:59
  //                                                      ^ d: lib/brew/chawan.dart:60
  //                                                                  ^ d: lib/brew/ceremony.dart:96
}

int bound<T extends Chawan>(T t) => t.heft() + (t + t).tilt();
//                                    ^ d: picker lib/brew/chawan.dart:18, lib/brew/old_chawan.dart:3; want lib/brew/chawan.dart:18 (#1220)

int extended(String name, List<Chawan> chawans) => name.frothed.length + chawans.length;
//                                                      ^ d: lib/brew/chawan.dart:47

int neighbours(Whisk w, Chawan? spare) {
  Matcha local() => Matcha();
  final either = spare ?? w.froth();
  final built = Whisk().froth();
  return either.tilt() + built.heft() + local().tilt();
  //            ^ d: picker lib/brew/chawan.dart:16, lib/brew/chawan.dart:34, lib/brew/old_chawan.dart:2
  //                           ^ d: lib/brew/chawan.dart:18
  //                                            ^ d: picker lib/brew/chawan.dart:16, lib/brew/chawan.dart:34, lib/brew/old_chawan.dart:2
}

class Pourer {
  Matcha makeChawan() => Matcha();
  int pour() => makeChawan().tilt();
  //                         ^ d: picker lib/brew/chawan.dart:16, lib/brew/chawan.dart:34, lib/brew/old_chawan.dart:2
}
