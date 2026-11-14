import 'kylix.dart';
import 'vessels.dart';

void feast() {
  final c = Kylix();
  c.libate();
  //^ d: lib/kylix.dart:1
  final s = Skyphos();
  s.libate();
  //^ d: lib/vessels.dart:1
  final r = Rhyton();
  r.libate();
  //^ d: lib/vessels.dart:2
  final k = Kantharos();
  k.libate();
  //^ d: lib/vessels.dart:4
}
