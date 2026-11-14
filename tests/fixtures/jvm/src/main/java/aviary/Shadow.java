package aviary;

import static aviary.Corvid.*;

// A static import gives way to the enum the code sits in, and to the type a switch is on (#1448).
enum Shadow {
    ROOK,
    KNIGHT;

    Shadow mine() {
        return ROOK;
            // ^ d: !jump
    }

    int unknown(Bishop piece) {
        return switch (piece) {
            case ROOK -> 1;
              // ^ d: none
            default -> 0;
        };
    }
}
