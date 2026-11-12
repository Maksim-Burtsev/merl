class Mug {}

class Cup {}

mixin Wash {}

class Tray with Wash {}

extension MugRinse on Mug {
  void rinse() {}
  bool get clean => false;
}

extension CupRinse on Cup {
  void rinse() {}
  bool get clean => true;
}

extension WashBuff on Wash {
  void buff() {}
}

extension MugBuff on Mug {
  void buff() {}
}
