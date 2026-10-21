class Leaf {
  int unfurl() => 1;
}

class Bag {
  int unfurl() => 2;
}

class Infuser {
  Leaf get leaf => Leaf();
  Infuser rinse(int times) => this;
  T hold<T>(T t) => t;
}

int steepAll(Infuser infuser) {
  final unfurled = infuser
      .rinse(1)
      .leaf
      .unfurl();
  //   ^ d: lib/infuser.dart:2
  //     status: via Leaf
  return unfurled + infuser.rinse(2).leaf.unfurl() + infuser.hold(Bag()).unfurl();
  //                                      ^ d: lib/infuser.dart:2
  //                                                                       ^ d: !jump
}
