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
  //^ d: picker lib/carafe/carafe.dart:8, lib/carafe/carafe.dart:16, lib/kwcafe/pot.dart:9, lib/saucer.dart:79; want lib/carafe/carafe.dart:8 (#1469)
  Missing.parse('1').weigh(500);
  //                 ^ d: !jump
}
