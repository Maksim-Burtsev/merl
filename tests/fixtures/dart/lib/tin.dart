class Tin<T> {
  const Tin(this.leaves, this.grade);

  final List<T> leaves;

  final String grade;

  String show() {
    return 'Tin{leaves: $leaves, grade: ${grade}}';
    //                   ^ d: lib/tin.dart:4
  }

  String plain(String other) {
    return leaves.toString() + grade + other;
    //                         ^ d: lib/tin.dart:6
  }

  String shadowed(String grade) {
    return 'grade: $grade';
    //              ^ d: lib/tin.dart:18
  }

  Tin<T> relabel(
    String grade,
    //     ^ d: !jump
  );
}

class Label {
  final String leaves = '';
  final String grade = '';
}

class Sprig {
  int swirl() => 1;
}

class Weed {
  int swirl() => 2;
}

class Caddie {
  final leaves = <Sprig>[];
  final List<Sprig> spares = [];

  int brew(List<Weed> rivals) {
    var total = 0;
    for (final leaf in leaves) {
      total += leaf.swirl();
      //            ^ d: lib/tin.dart:35
    }
    for (var spare in spares) {
      total += spare.swirl();
      //             ^ d: lib/tin.dart:35
    }
    for (final rival in rivals) {
      total += rival.swirl();
      //             ^ d: lib/tin.dart:39
    }
    final loose = [];
    for (final stray in loose) {
      total += stray.swirl();
      //             ^ d: !jump
    }
    return total;
  }
}

abstract class _Urn {
  final _leaf = Sprig();
}

class Urn extends _Urn {
  int pour() => _leaf.swirl();
  //                  ^ d: lib/tin.dart:35
}

// A type argument is no call, though a call follows on the line (#1338).
Future<Tin<int>> packed<S>() async => Tin([1], 'green');
//     ^ d: lib/tin.dart:1
//                                    ^ d: picker lib/tin.dart:1, lib/tin.dart:2
