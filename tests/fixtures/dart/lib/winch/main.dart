import 'pawl.dart';
import 'winch.dart';

void run(Winch w) {
  w.gear.click();
//       ^ d: lib/winch/pawl.dart:2
  w.slack.click();
//        ^ d: lib/winch/pawl.dart:2
  w.slack = Pawl();
//  ^ d: lib/winch/winch.dart:7
  final x = Winch();
  x.slack.click();
//        ^ d: lib/winch/pawl.dart:2
  final s = w.slack;
  s.click();
//  ^ d: lib/winch/pawl.dart:2
  final g = w.gear;
  g.click();
//  ^ d: lib/winch/pawl.dart:2
  var h = x.slack;
  h.click();
//  ^ d: lib/winch/pawl.dart:2
  final l = w.loose;
  l.click();
//  ^ d: picker lib/winch/pawl.dart:2, lib/winch/pawl.dart:6
  w.loose.click();
//        ^ d: picker lib/winch/pawl.dart:2, lib/winch/pawl.dart:6
}

void lift(Crane c) {
  c.w.slack = Pawl();
//    ^ d: lib/winch/winch.dart:7
  final s = c.w.slack;
  s.click();
//  ^ d: lib/winch/pawl.dart:2
}
