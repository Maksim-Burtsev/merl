class Stylus {
  String blot() => 'stylus';
}

class Nib {
  String blot() => 'nib';
}

String a(Stylus s) => 's.blot()' + "s.blot()";
//                       ^ d: picker lib/quoted.dart:2, lib/quoted.dart:6
//                                    ^ d: picker lib/quoted.dart:2, lib/quoted.dart:6

String b(Stylus s) => '${s.blot()} $s.blot() \${s.blot()}';
//                         ^ d: lib/quoted.dart:2
//                                    ^ d: picker lib/quoted.dart:2, lib/quoted.dart:6
//                                                ^ d: picker lib/quoted.dart:2, lib/quoted.dart:6

String c(Stylus s) => r'${s.blot()}';
//                          ^ d: picker lib/quoted.dart:2, lib/quoted.dart:6

String e(Stylus s) => '''
  s.blot()
//  ^ d: picker lib/quoted.dart:2, lib/quoted.dart:6
''';

String d(Stylus s) => 'a\\' + s.blot() + r'\' + s.blot(); // '
//                              ^ d: lib/quoted.dart:2
//                                                ^ d: lib/quoted.dart:2
