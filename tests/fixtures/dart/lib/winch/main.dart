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
//  ^ d: picker lib/winch/pawl.dart:2, lib/winch/pawl.dart:6; want lib/winch/pawl.dart:2 (#1483)
  w.loose.click();
//        ^ d: picker lib/winch/pawl.dart:2, lib/winch/pawl.dart:6
}
