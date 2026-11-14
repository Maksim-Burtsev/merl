import 'package:steep_annotation/steep_annotation.dart';
import 'package:steep_codec/steep_codec.dart';

// #1452: with two missing packages a dart:core name still offers neither import.
class Steep {
  final Duration wait = const Duration();
  //    ^ d: none
  final Timer? timer = null;
  //    ^ d: picker lib/tea/cored_twice.dart:1, lib/tea/cored_twice.dart:2
}
