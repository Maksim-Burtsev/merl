import 'carafe.dart';

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
