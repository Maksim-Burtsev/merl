import 'carafe.dart';

class Rack {
  final List<Carafe> carafes = [];

  void fill(List<Carafe> kettles, Map<String, Carafe> byName) {
    kettles[0].boil();
    //         ^ d: lib/carafe/carafe.dart:2
    byName['a']!.boil();
    //           ^ d: lib/carafe/carafe.dart:2
    byName['a']?.boil();
    //           ^ d: lib/carafe/carafe.dart:2
    carafes[0].boil();
    //         ^ d: lib/carafe/carafe.dart:2
  }

  void stack(List<List<Carafe>> grid, Map<String, List<Carafe>> rows) {
    grid[0][1].boil();
    //         ^ d: !jump
    rows['a']![0].boil();
    //            ^ d: !jump
  }
}

class Shelf<Carafe> {
  final List<Carafe> items = [];

  void fill(Map<String, Carafe> byName) {
    items[0].boil();
    //       ^ d: !jump
    byName['a']!.boil();
    //           ^ d: !jump
  }
}
