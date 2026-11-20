class Map<K, V> {
  Tumbler operator [](K key) => Tumbler();
}

class Tumbler {
  void stir() {}
}
