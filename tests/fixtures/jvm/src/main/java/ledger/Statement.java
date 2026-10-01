package ledger;

class Statement {
    static final String SHAPE = """
        case Crimson, Azure
        object Folio:
        """;

    long sum(scala.collection.immutable.List<Voucher> xs) {
    //                                       ^ d: src/main/scala/ledger/Kinds.scala:8
        return Folio.tally(xs);
        //     ^ d: src/main/scala/ledger/Kinds.scala:5
        //           ^ d: src/main/scala/ledger/Kinds.scala:6
    }

    int tone(Hue h) {
    //       ^ d: src/main/scala/ledger/Kinds.scala:43
        switch (h) {
            case Crimson:
            //   ^ d: src/main/scala/ledger/Kinds.scala:44
                return 1;
            case Azure, Olive:
            //          ^ d: src/main/scala/ledger/Kinds.scala:44
                return 2;
        }
        return switch (h) {
            case Crimson -> 3;
            //   ^ d: src/main/scala/ledger/Kinds.scala:44
            default -> 4;
        };
    }
}
