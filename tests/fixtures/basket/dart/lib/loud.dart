import 'ext.dart';

String loud() {
  final s = 'a';
  return s.shout();
  //       ^ d: dart/lib/ext.dart:2
}
