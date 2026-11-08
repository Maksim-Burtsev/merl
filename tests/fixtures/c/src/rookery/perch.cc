#include "corvid.hh"

// A case two enums of the project share, written without its enum (#1448).
int perch(Corvid bird) {
    switch (bird) {
    case Rook:
      // ^ d: src/rookery/corvid.hh:3
        return 1;
    case Corvid::Raven:
        return 0;
    }
    return 0;
}

int qualified(Corvid bird) {
    switch (bird) {
    case Corvid::Rook:
              // ^ d: src/rookery/corvid.hh:3
        return 1;
    default:
        return 0;
    }
}

Corvid plain() { return Rook; }
                     // ^ d: src/rookery/corvid.hh:3
