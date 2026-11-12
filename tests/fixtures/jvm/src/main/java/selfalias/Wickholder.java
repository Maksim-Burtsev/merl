package selfalias;

public class Wickholder {
    int flare;
    class Inner { int flare; }
    void go() {
        final Wickholder self = this;
        Runnable r = new Runnable() {
            int flare;
            public void run() {
                this.flare++;
//                   ^ d: src/main/java/selfalias/Wickholder.java:9
                self.flare++;
//                   ^ d: picker src/main/java/selfalias/Wickholder.java:4
            }
        };
    }
}
