package amphora;

public class AmphoraRack<Amphora> {
//                       ^ d: src/main/java/amphora/AmphoraRack.java:3
    Amphora held;
  //^ d: src/main/java/amphora/AmphoraRack.java:3

    Amphora take(Amphora x) {
  //^ d: src/main/java/amphora/AmphoraRack.java:3
    //           ^ d: src/main/java/amphora/AmphoraRack.java:3
        return x;
    }

    <Amphora> void own(Amphora x) {
   //^ d: src/main/java/amphora/AmphoraRack.java:14
    //                 ^ d: src/main/java/amphora/AmphoraRack.java:14
    }

    static <Amphora> Amphora one(Amphora x) {
    //      ^ d: src/main/java/amphora/AmphoraRack.java:19
    //               ^ d: src/main/java/amphora/AmphoraRack.java:19
    //                           ^ d: src/main/java/amphora/AmphoraRack.java:19
        return x;
    }
}

class AmphoraPlain {
    Amphora plain(Amphora x) {
  //^ d: src/main/java/amphora/Amphora.java:3
    //            ^ d: src/main/java/amphora/Amphora.java:3
        return x;
    }
}

class AmphoraBound<T extends Amphora> {
//                 ^ d: src/main/java/amphora/AmphoraRack.java:35
//                           ^ d: src/main/java/amphora/Amphora.java:3
    T held;
  //^ d: src/main/java/amphora/AmphoraRack.java:35
}

interface AmphoraHolder<Amphora> {
//                      ^ d: src/main/java/amphora/AmphoraRack.java:42
    Amphora get();
  //^ d: src/main/java/amphora/AmphoraRack.java:42
}

class AmphoraRange {
    static final int LOW = 0;
    static final int HIGH = 9;

    <Amphora> void own(Amphora x) {}

    /**
     * @param <Amphora> the kind it holds.
//             ^ d: src/main/java/amphora/Amphora.java:3
     */
    boolean in(int a) {
        // an Amphora counts
//            ^ d: src/main/java/amphora/Amphora.java:3
        if (a < LOW || a > HIGH) {
//              ^ d: src/main/java/amphora/AmphoraRack.java:49
            return LOW == a;
//                 ^ d: src/main/java/amphora/AmphoraRack.java:49
        }
        while (a < HIGH && a > 1) {
            a += HIGH;
//               ^ d: src/main/java/amphora/AmphoraRack.java:50
        }
        return false;
    }
}

class AmphoraWrapped<
        Amphora> {
  //    ^ d: src/main/java/amphora/AmphoraRack.java:75
    Amphora held;
  //^ d: src/main/java/amphora/AmphoraRack.java:75

    <Amphora> Amphora wrapped(
            Amphora x,
    //      ^ d: src/main/java/amphora/AmphoraRack.java:80
            int y) {
        return x;
    }

    Amphora plain(
  //^ d: src/main/java/amphora/AmphoraRack.java:75
            Amphora x) {
    //      ^ d: src/main/java/amphora/AmphoraRack.java:75
        return x;
    }
}

class AmphoraWrappedOutside {
    Amphora plain(
  //^ d: src/main/java/amphora/Amphora.java:3
            Amphora x) {
    //      ^ d: src/main/java/amphora/Amphora.java:3
        return x;
    }

    public static <Amphora>
            Amphora first(Amphora a) {
    //      ^ d: src/main/java/amphora/AmphoraRack.java:103
    //                    ^ d: src/main/java/amphora/AmphoraRack.java:103
        return a;
    }
}

class AmphoraCrossBound<
        T extends Amphora,
        Amphora
> {
    Amphora held;
  //^ d: src/main/java/amphora/AmphoraRack.java:113
}
