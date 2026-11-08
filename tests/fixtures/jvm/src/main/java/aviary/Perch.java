package aviary;

import static aviary.Corvid.*;

// A case two enums of the project share, written without its enum (#1448).
class Perch {
    int perch(Corvid bird) {
        switch (bird) {
            case ROOK:
              // ^ d: none; want src/main/java/aviary/Corvid.java:5 (#1448)
                return 1;
            default:
                return 0;
        }
    }

    int arrow(Chessman piece) {
        return switch (piece) {
            case ROOK -> 1;
              // ^ d: none; want src/main/java/aviary/Chessman.java:4 (#1448)
            case PAWN -> 0;
        };
    }

    Corvid plain() {
        Corvid qualified = Corvid.ROOK;
                               // ^ d: src/main/java/aviary/Corvid.java:5
        return ROOK;
            // ^ d: none; want src/main/java/aviary/Corvid.java:5 (#1448)
    }
}
