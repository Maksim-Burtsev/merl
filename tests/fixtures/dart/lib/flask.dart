import 'dart:async';

class Bloom {
  int petal() => 1;
}

class Thorn {
  int petal() => 2;
}

class Pod<T> {
  Pod.of(Future<T> seed);
  Pod.wrap(T seed);

  Future<T> get ripe => throw 0;
  Future<T?> get later => throw 0;
  Future<Bloom> get bloom => throw 0;
  FutureOr<Thorn> get either => throw 0;
  dynamic get anything => null;
}

class Grower {
  Future<Thorn> grow(int n) async => Thorn();
  Future<S> raise<S>(S seed) async => seed;
}

class Garden {
  final grower = Grower();

  Future<int> tend() async {
    final pod = Pod.of(
      grower.grow(
        1,
      ),
    );
    final thorn = await pod.ripe;
    var total = thorn.petal();
    //                ^ d: lib/flask.dart:8
    final later = await pod.later;
    total += later?.petal() ?? 0;
    //              ^ d: lib/flask.dart:8
    final bloom = await pod.bloom;
    total += bloom.petal();
    //             ^ d: lib/flask.dart:4
    final either = await pod.either;
    total += either.petal();
    //              ^ d: lib/flask.dart:8
    final anything = await pod.anything;
    total += anything.petal();
    //                ^ d: !jump
    final wrapped = Pod.wrap(Bloom());
    final loose = await wrapped.ripe;
    total += loose.petal();
    //             ^ d: !jump
    final raised = Pod.of(grower.raise(Bloom()));
    final guess = await raised.ripe;
    return total + guess.petal();
    //                   ^ d: !jump
  }
}
