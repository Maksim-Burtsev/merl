class Gridiron {
  int sear() => 2;

  List<int> scoped(List<int> items) {
    final a = items.map((i) => this.sear()).toList();
    //                              ^ d: lib/thisbound/gridiron.dart:2
    final b = items.map((i) { return sear(); }).toList();
    //                               ^ d: lib/thisbound/gridiron.dart:2
    return a + b;
  }
}
