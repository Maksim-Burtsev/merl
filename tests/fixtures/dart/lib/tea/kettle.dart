// #931: what Dart's own scoping makes hard for `d`.
library;

part 'kettle_parts.dart';

class Kettle {
  Kettle(this.cups);
  Kettle.cold() : cups = 0;
  Kettle.boiled(this.cups);
  Kettle.again() : this.cold();
  //                    ^ d: picker lib/tea/kettle.dart:8, lib/tea/table.dart:10, lib/tea/teapot.dart:4; want lib/tea/kettle.dart:8 (#949)
  factory Kettle.from(int n) => Kettle.boiled(n);
  //                                   ^ d: lib/tea/kettle.dart:9

  int cups;

  int pour(int cups) {
    this.cups -= cups;
    //   ^ d: lib/tea/kettle.dart:15
    //           ^ d: lib/tea/kettle.dart:17
    return steep(cups) + _Spout().drip;
    //     ^ d: lib/tea/kettle_parts.dart:3
    //                   ^ d: lib/tea/kettle_parts.dart:7
    //                            ^ d: lib/tea/kettle_parts.dart:8
  }
}
