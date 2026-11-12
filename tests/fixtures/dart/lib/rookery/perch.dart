import 'chessman.dart';
import 'corvid.dart';

// A case two enums of the project share, written without its enum (#1448).
int perch(Corvid bird) {
  switch (bird) {
    case Corvid.rook:
             // ^ d: lib/rookery/corvid.dart:1
      return 1;
    case Corvid.raven:
      return 0;
  }
}

int advance(Chessman piece) => switch (piece) {
      Chessman.rook => 1,
            // ^ d: lib/rookery/chessman.dart:1
      Chessman.pawn => 0,
    };

bool shorthand(Corvid bird) => bird == .rook;
                                     // ^ d: lib/rookery/corvid.dart:1

bool plain(Chessman piece) => piece != .rook;
                                     // ^ d: lib/rookery/chessman.dart:1

bool untyped(List<Chessman> pieces) {
  for (final piece in pieces) {
    if (piece == .rook) return true;
                // ^ d: lib/rookery/chessman.dart:1
  }
  return false;
}

bool shadowed(Corvid bird) {
  if (bird == .raven) {
    final bird = Chessman.pawn;
    return bird == .rook;
                 // ^ d: picker lib/rookery/chessman.dart:1, lib/rookery/corvid.dart:1
  }
  return false;
}
