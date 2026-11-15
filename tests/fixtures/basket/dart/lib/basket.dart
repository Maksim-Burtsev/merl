class Basket {
  final List<String> items = [];

  void add(String item) {
    items.add(item);
  }

  bool contains(String item) => items.contains(item);
}
