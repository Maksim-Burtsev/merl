import 'carafe.dart';
import '../courier.dart' show weigh;

class Pot {
  void boil() {}
}

void pour() {
  Carafe.builder().size(2).finish().boil();
  //     ^ d: lib/carafe/carafe.dart:4
  //               ^ d: lib/carafe/carafe.dart:8
  //                       ^ d: lib/carafe/carafe.dart:10
  //                                ^ d: lib/carafe/carafe.dart:2
  Carafe.builder()
      .size(2)
      //^ d: lib/carafe/carafe.dart:8
      .finish()
      //^ d: lib/carafe/carafe.dart:10
      .boil();
      //^ d: lib/carafe/carafe.dart:2
  Mystery.builder().size(2).boil();
  //                ^ d: !jump
  //                        ^ d: !jump
  Pot().boil();
}

void stock() {
  final b = Carafe.builder();
  b.size(2);
  //^ d: lib/carafe/carafe.dart:8
  final c = Carafe.builder().size(2);
  c.finish();
  //^ d: lib/carafe/carafe.dart:10
  final m = Mystery.builder();
  m.size(2);
  //^ d: !jump
  Missing.parse('1').weigh(500);
  //                 ^ d: !jump
}
