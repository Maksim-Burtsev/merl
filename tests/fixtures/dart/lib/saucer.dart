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

// A field or a local whose type is a function type (#1087).
class Ladle {
  Ladle(this.logPrint, this._builder);

  void Function(Object object) logPrint;
  final Stream<List<int>> Function() _builder;

  void serve() {
    final void Function() pour = () {};
    logPrint(_builder());
    //^ d: lib/saucer.dart:53
    //         ^ d: lib/saucer.dart:54
    pour();
    //^ d: lib/saucer.dart:57
  }
}

// A named parameter list closed by `})` before a constructor's initializer list (#1084).
class Cup {
  Cup({
    String? tint,
    int? size,
  })  : assert(tint != 'x'),
        size = size ?? 0 {
      //^ d: lib/saucer.dart:78
    print(tint);
    //    ^ d: lib/saucer.dart:69
  }

  int size;
  String? tint;

  void spill(bool full) {
    final m = full
        ? Cup(
            tint: tint,
          ) : {
            'b': tint,
            //   ^ d: picker lib/saucer.dart:20, lib/saucer.dart:79
          };
    print(m);
  }
}
