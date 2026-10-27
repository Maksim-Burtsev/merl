class Scribe {
  void note(String s) {}
}

class Base {
  final _ink = Scribe();
  final pen = Scribe();
  final nib = Scribe();
}
