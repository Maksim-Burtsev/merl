import 'tub.dart';
import 'tun.dart';

void cast(Object v) {
  (v as Tun).dunk();
  //         ^ d: lib/tubs/tun.dart:2
  (v as Tub?)?.slop();
  //           ^ d: lib/tubs/tub.dart:3
  final k = v as Tun;
  k.slop();
  //^ d: lib/tubs/tun.dart:3
}
