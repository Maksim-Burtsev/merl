import 'chessman.dart';
import 'corvid.dart';

// A case two enums of the project share, written without its enum (#1448).
int perch(Corvid bird) {
  switch (bird) {
    case Corvid.rook:
             // ^ d: picker lib/rookery/chessman.dart:1, lib/rookery/corvid.dart:1; want lib/rookery/corvid.dart:1 (#1448)
      return 1;
    case Corvid.raven:
      return 0;
  }
}

int advance(Chessman piece) => switch (piece) {
      Chessman.rook => 1,
            // ^ d: picker lib/rookery/chessman.dart:1, lib/rookery/corvid.dart:1; want lib/rookery/chessman.dart:1 (#1448)
      Chessman.pawn => 0,
    };

bool shorthand(Corvid bird) => bird == .rook;
                                     // ^ d: picker lib/rookery/chessman.dart:1, lib/rookery/corvid.dart:1; want lib/rookery/corvid.dart:1 (#1448)
