import 'kettle.dart';
import 'teapot.dart';
import 'stale/kettle.dart' as stale;

// The review of #1022: where the new rules of #949 and #950 must stay off.
// A named argument names the callee's field, not the function of its name.
class Gauge {
  final int level;
  Gauge({required this.level});
}

int level(int x) => x;

class Cup {
  Gauge fill() {
    return Gauge(level: level(2));
    //           ^ d: picker lib/tea/cup.dart:8, lib/tea/cup.dart:12
    //                  ^ d: lib/tea/cup.dart:12
  }

  // A field named like an import prefix hides it in its class.
  int stale = 0;
  String mark() => stale.toString();
  //               ^ d: lib/tea/cup.dart:22
}
