part of 'kettle.dart';

int steep(int cups) => cups * 2;

const steeped = 3;

class _Spout {
  int get drip => steeped;
  //              ^ d: lib/tea/kettle_parts.dart:5
}
