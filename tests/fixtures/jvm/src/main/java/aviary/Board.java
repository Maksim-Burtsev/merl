package aviary;

class Board {
    int advance(Chessman piece) {
        switch (piece) {
            case ROOK:
              // ^ d: none; want src/main/java/aviary/Chessman.java:4 (#1448)
                return 1;
            default:
                return 0;
        }
    }
}
