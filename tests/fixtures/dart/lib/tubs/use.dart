import 'tub.dart';
import 'tun.dart';

final c = Tun();

void go(List<Tub> tubs) {
  tubs.forEach((x) => x.dunk());
  //                    ^ d: lib/tubs/tub.dart:2
  tubs.map((x) => x.dunk());
  //                ^ d: lib/tubs/tub.dart:2
  tubs.where((x) => x.dunk());
  //                  ^ d: lib/tubs/tub.dart:2
  tubs.forEach((c) { c.dunk(); });
  //                   ^ d: lib/tubs/tub.dart:2
  tubs.sort((a, b) => a.slop() - b.slop());
  //                    ^ d: lib/tubs/tub.dart:3
  //                               ^ d: lib/tubs/tub.dart:3
  tubs.forEach((Tub x) => x.dunk());
  //                        ^ d: lib/tubs/tub.dart:2
  tubs.any((x) => x.dunk());
  //                ^ d: lib/tubs/tub.dart:2
  tubs.forEach((x) { final x2 = Tun(); x2.dunk(); });
  //                                      ^ d: lib/tubs/tun.dart:2
}

void each(void Function(Tub) f) {}

void run() {
  each((x) => x.dunk());
  //            ^ d: picker lib/tubs/hogshead.dart:2, lib/tubs/tub.dart:2, lib/tubs/tun.dart:2; want lib/tubs/tub.dart:2 (#1488)
  c.dunk();
  //^ d: !jump
}

class Holder {
  final List<Tub> tubs = [];
  void a() => tubs.forEach((x) => x.dunk());
  //                                ^ d: lib/tubs/tub.dart:2
  void b() => this.tubs.forEach((x) => x.dunk());
  //                                     ^ d: lib/tubs/tub.dart:2
}

void shadowed(List<Tub> tubs, List<Tun> tuns) {
  tuns.forEach((x) => x.dunk());
  //                    ^ d: lib/tubs/tun.dart:2
  tubs.forEach((x) => tuns.forEach((y) => x.dunk()));
  //                                        ^ d: lib/tubs/tub.dart:2
  tubs.map((x, y) => x.dunk());
  //                   ^ d: !jump
}

void loose(List tubs) {
  tubs.forEach((x) => x.dunk());
  //                    ^ d: !jump
}

class Shade {
  final List<Tub> tubs = [];
  void a(List<Tun> tubs) => this.tubs.forEach((x) => x.dunk());
  //                                                   ^ d: !jump
  void b(List<Tun> tubs) => tubs.forEach((x) => x.dunk());
  //                                              ^ d: lib/tubs/tun.dart:2
}
