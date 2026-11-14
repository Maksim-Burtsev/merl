import 'package:steep_annotation/steep_annotation.dart';

// #1452: `steep_annotation` is not in the pub cache, yet a name of dart:core is never the
// import's: `Duration` and `print` are the SDK's, out of reach here, never the import line nor
// the namesakes in `rigs/clock.dart`. `Timer` is not dart:core's, so the import may give it.
class Brew {
  final Duration wait = const Duration();
  //    ^ d: none
  final String name = '';
  //    ^ d: none
  final Timer? timer = null;
  //    ^ d: lib/tea/cored.dart:1

  void pour() => print(name);
  //             ^ d: none
}
