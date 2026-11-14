import 'cups.dart';
import 'shine.dart';

void use(Cup cup, Tray tray) {
  final mug = Mug();
  cup.rinse();
  //  ^ d: lib/rinse/cups.dart:15
  mug.rinse();
  //  ^ d: lib/rinse/cups.dart:10
  cup.clean;
  //  ^ d: lib/rinse/cups.dart:16
  tray.buff();
  //   ^ d: lib/rinse/cups.dart:20
  cup.gleam();
  //  ^ d: lib/rinse/shine.dart:4
}
