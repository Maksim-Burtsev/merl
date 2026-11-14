// #1452: every import resolves and none gives `Duration`, so it is dart:core's, never the
// namesake in `clock.dart`, which this file does not import.
class Clocked {
  final Duration wait = const Duration();
  //    ^ d: none
}
