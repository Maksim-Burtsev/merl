class Saucer {
  int rim = 0;
}

void stack(List<Saucer> saucers) {
  final chipped = saucers.isEmpty;
  print(chipped);
}

void wash(Saucer saucer) {
  // A member is never a function's local (#1083).
  saucer.chipped;
  //     ^ d: none
  // From `lib/`, a declaration under the package's `test/` is out of reach (#1083).
  saucer.polish();
  //     ^ d: none
}

class Glaze {
  String? tint;
}

void fire(
  String kiln, {
  String? tint,
}) {
  for (final s in [kiln]) {
    print(tint);
    //    ^ d: lib/saucer.dart:25
  }
}

void cool(
  String kiln, [
  String? tint,
]) =>
    print(tint);
    //    ^ d: lib/saucer.dart:35

void glow(Future<int> f) {
  f.then((v) {
    print(v);
  }).then((w) {
    print(w);
    //    ^ d: lib/saucer.dart:43
  });
}
