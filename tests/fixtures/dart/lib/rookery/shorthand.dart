import 'chessman.dart';
import 'corvid.dart';

// Dot shorthands whose context type is written down (#1448).
int land(Corvid bird) => 0;

int nest(int depth, {required Chessman piece, Corvid? bird}) => depth;

int hop(Object any) => 0;

int fly() => land(.rook);
                // ^ d: lib/rookery/corvid.dart:1

int settle() => nest(1, piece: .rook);
                             // ^ d: lib/rookery/chessman.dart:1

int roost() => nest(2, bird: .rook, piece: .pawn);
                            // ^ d: lib/rookery/corvid.dart:1

int wander() => hop(.rook);
                  // ^ d: picker lib/rookery/chessman.dart:1, lib/rookery/corvid.dart:1

int sort(Corvid bird) {
  switch (bird) {
    case .rook:
       // ^ d: lib/rookery/corvid.dart:1
      return 1;
    case .raven:
      return 0;
  }
}

int rank(Chessman piece) {
  return switch (piece) {
    .rook => 1,
  // ^ d: picker lib/rookery/chessman.dart:1, lib/rookery/corvid.dart:1; want lib/rookery/chessman.dart:1 (#1448)
    .pawn => 0,
  };
}

int grade(Chessman piece) => switch (piece) {
      .rook => 1,
    // ^ d: picker lib/rookery/chessman.dart:1, lib/rookery/corvid.dart:1; want lib/rookery/chessman.dart:1 (#1448)
      .pawn => 0,
    };

Corvid pick(bool dark) {
  if (dark) {
    return .raven;
  }
  return .rook;
       // ^ d: lib/rookery/corvid.dart:1
}

Chessman first() => .rook;
                   // ^ d: lib/rookery/chessman.dart:1

Corvid wrapped() {
  final make = () {
    return .rook;
         // ^ d: picker lib/rookery/chessman.dart:1, lib/rookery/corvid.dart:1
  };
  make();
  return .raven;
}

class Box<Chessman> {
  Chessman keep() {
    return .rook;
         // ^ d: picker lib/rookery/chessman.dart:1, lib/rookery/corvid.dart:1
  }
}

class Coop {
  int land(Chessman piece) => 1;

  int perch() => land(.rook);
                    // ^ d: picker lib/rookery/chessman.dart:1, lib/rookery/corvid.dart:1
}

int vague(dynamic seen) {
  final bird = seen;
  switch (bird) {
    case .rook:
       // ^ d: picker lib/rookery/chessman.dart:1, lib/rookery/corvid.dart:1
      return 1;
  }
  return 0;
}
