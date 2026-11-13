package selfalias;

public class Bellows {
    int puff;
    class Inner {
        int puff;
        void go() {
            Bellows.this.puff++;
//                       ^ d: src/main/java/selfalias/Bellows.java:4
            Inner.this.puff++;
//                     ^ d: src/main/java/selfalias/Bellows.java:6
            this.puff++;
//               ^ d: src/main/java/selfalias/Bellows.java:6
            Runnable r = new Runnable() {
                int puff;
                public void run() {
                    Bellows.this.puff++;
//                               ^ d: src/main/java/selfalias/Bellows.java:4
                    this.puff++;
//                       ^ d: src/main/java/selfalias/Bellows.java:15
                }
            };
        }
    }
}
