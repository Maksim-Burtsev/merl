import 'base.dart';

class Quill {
  void note(String s) {}
}

final pen = Quill();

class Child extends Base {
  void write() {
    pen.note('a');
    //  ^ d: picker lib/vat/child.dart:4, lib/vat/base.dart:2; want lib/vat/child.dart:4 (#1299)
    _ink.note('b');
    //   ^ d: picker lib/vat/child.dart:4, lib/vat/base.dart:2
    nib.note('c');
    //  ^ d: lib/vat/base.dart:2
  }
}
