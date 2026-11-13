import 'pots.dart';

extension on Pot {
  void scrub() {}
}

extension on Tin {
  void scrub() {}
}

extension BoxSeal<T> on Box<T> {
  void seal() {}
}

extension JarSeal<T> on Jar<T> {
  void seal() {}
}

extension<T> on Box<T> {
  void tip() {}
}

extension<T> on Jar<T> {
  void tip() {}
}

extension Wipe<T extends Pot> on T {
  void wipe() {}
}

extension PanWipe on Pan {
  void wipe() {}
}

void use(Pot pot, Pan pan, Box<int> box, Jar<String> jar) {
  pot.scrub();
  //  ^ d: lib/scrub/use.dart:4
  box.seal();
  //  ^ d: lib/scrub/use.dart:12
  jar.seal();
  //  ^ d: lib/scrub/use.dart:16
  box.tip();
  //  ^ d: lib/scrub/use.dart:20
  jar.tip();
  //  ^ d: lib/scrub/use.dart:24
  pan.wipe();
  //  ^ d: lib/scrub/use.dart:32
  pot.wipe();
  //  ^ d: !jump
}
