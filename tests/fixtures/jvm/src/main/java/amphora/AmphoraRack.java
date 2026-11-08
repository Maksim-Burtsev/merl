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
