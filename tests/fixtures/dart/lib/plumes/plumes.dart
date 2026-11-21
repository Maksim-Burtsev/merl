class Plume {
  void scrawl() {}
}

class Blotter {
  void scrawl() {}
}

class Inkstand {
  final List<Blotter> plumes = [];
}

// `other.plumes[0]` reads the other class's field, never this class's `plumes` (#1610).
class Desk {
  final List<Plume> plumes = [];
  final Inkstand other = Inkstand();

  void write() {
    other.plumes[0].scrawl();
    //              ^ d: !jump
  }
}
