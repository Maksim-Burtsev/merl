package aviary;

import static zoo.Corvid.*;

// A static import of an enum the project does not hold names no case of its namesake (#1448).
class Stray {
    Object stray() {
        return ROOK;
            // ^ d: none
    }
}
