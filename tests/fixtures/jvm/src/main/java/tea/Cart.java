package tea;

public class Cart {
    public int roll() {
        return new Tray(2).tip() + new Tray().tip();
        //         ^ d: src/main/java/tea/Cart.java:16
        //                 ^ d: src/main/java/tea/Cart.java:19
        //                             ^ d: src/main/java/tea/Cart.java:13
        //                                    ^ d: src/main/java/tea/Cart.java:19
    }

    static class Tray {
        Tray() {
        }

        Tray(int legs) {
        }

        int tip() {
            return 2;
        }
    }
}
