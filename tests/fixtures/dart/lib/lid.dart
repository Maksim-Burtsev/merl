class Lid {
  final int _value;
  Lid(int value) : _value = value;
  //                        ^ d: lib/lid.dart:3
  int get value => _value;
}

class Seal {
  final int _value;
  Seal(int value)
      : _value = value;
  //             ^ d: lib/lid.dart:10
  int get value => _value;

  void check(int? x) {
    if (x case final value?) { print(value); }
    //                                ^ d: lib/lid.dart:16
    if (x case final value?) {
      print(value);
      //    ^ d: lib/lid.dart:18
    }
  }
}
