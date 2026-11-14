package aviary;

class Board {
    int advance(Chessman piece) {
        switch (piece) {
            case ROOK:
              // ^ d: src/main/java/aviary/Chessman.java:4
                return 1;
            default:
                return 0;
        }
    }
}
