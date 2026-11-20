import 'carafe.dart';
import 'own_map.dart';

void fill(Map<String, Carafe> byName) {
  byName['a'].boil();
  //          ^ d: !jump
}
