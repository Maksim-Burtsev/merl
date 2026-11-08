#include "chessman.hh"

int advance(Chessman piece) {
    switch (piece) {
    case Rook:
      // ^ d: src/rookery/chessman.hh:3
        return 1;
    default:
        return 0;
    }
}
