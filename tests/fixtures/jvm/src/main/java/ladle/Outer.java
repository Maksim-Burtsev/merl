package ladle;

import java.util.List;

public class Outer {
    private List<Mug> items;

    void a() { items.forEach(c -> c.fill()); }
    //                              ^ d: src/main/java/ladle/Mug.java:4

    class Inner extends Base {
        void b() { items.forEach(c -> c.fill()); }
        //                              ^ d: !jump
        void d() { this.items.forEach(c -> c.fill()); }
        //                                   ^ d: !jump
    }

    void e() {
        Runnable r = new Base() {
            public void run() { items.forEach(c -> c.fill()); }
            //                                       ^ d: !jump
        };
    }

    void f() {
        List<Cup>
            items = List.of();
        items.forEach(c -> c.fill());
        //                   ^ d: !jump
    }

    void g(
            List<Cup> items) {
        items.forEach(c -> c.fill());
        //                   ^ d: src/main/java/ladle/Cup.java:4
    }
}
