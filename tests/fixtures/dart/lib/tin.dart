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
