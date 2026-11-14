import 'pawl.dart';

class Winch {
  Pawl get gear => Pawl();
  Pawl _slack = Pawl();
  Pawl get slack => _slack;
  set slack(Pawl v) => _slack = v;
  get loose => _slack;
  set loose(v) => _slack = v;

  void grow() {
    slack.click();
//        ^ d: lib/winch/pawl.dart:2
    slack = Pawl();
//  ^ d: lib/winch/winch.dart:7
  }
}

class Crane {
  Winch _w = Winch();
  Winch get w => _w;
  set w(Winch v) => _w = v;
}
