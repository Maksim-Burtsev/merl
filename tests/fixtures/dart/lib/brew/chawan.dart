// #1126: the types the third corner reaches.
mixin Scoopable {
  int scoop() => 1;
}

abstract class Rinsable {
  int rinseOut();
}

class Chawan with Scoopable implements Rinsable {
  Chawan();
  Chawan.dry() : glaze = 0;

  int glaze = 1;

  int tilt() => glaze;

  int heft() => 2;

  @override
  int rinseOut() => 0;

  String trim() => 'chawan';

  int get inMinutes => 3;

  void write(String s) {}

  Chawan operator +(Chawan other) => this;
}

class Matcha extends Chawan {
  @override
  int tilt() => super.tilt() + 1;
}

class Whisk {
  Whisk dip(int depth) => this;
  Chawan froth() => Chawan();
  int call(int n) => n;
  static Whisk bamboo() => Whisk();
}

Chawan makeChawan() => Chawan();

extension Frothy on String {
  String get frothed => this;
}

extension Tilted on Chawan {
  int tiltTwice() => tilt() * 2;
}

enum Grade {
  ceremonial(3),
  culinary(1);

  const Grade(this.score);
  final int score;
  int doubled() => score * 2;
}

extension Swirled<T> on Iterable<T> {
  T? firstSwirl() => isEmpty ? null : first;
}

extension Stamped on Object {
  String stamp() => '$this';
}

extension Dusted on Duration {
  int dust() => inSeconds;
}
