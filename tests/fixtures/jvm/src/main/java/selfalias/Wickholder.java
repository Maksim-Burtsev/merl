package selfalias;

public class Wickholder {
    int flare;
    class Inner { int flare; }
    void go() {
        Runnable r = new Runnable() {
            int flare;
            public void run() {
                this.flare++;
//                   ^ d: src/main/java/selfalias/Wickholder.java:8
            }
        };
    }
}
