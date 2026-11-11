package ladle;

import java.util.List;

public class Use {
    private List<Cup> cups;
    List loose;
    Mug c;

    void a() { cups.forEach(c -> c.fill()); }
    //                             ^ d: src/main/java/ladle/Cup.java:4
    void b() { this.cups.forEach(x -> x.fill()); }
    //                                  ^ d: src/main/java/ladle/Cup.java:4
    void d() { cups.stream().map(x -> x.fill()); }
    //                                  ^ d: src/main/java/ladle/Cup.java:4
    void e() { cups.stream().filter(x -> x.fill()).count(); }
    //                                     ^ d: src/main/java/ladle/Cup.java:4
    void f() { cups.sort((x, y) -> x.size() - y.size()); }
    //                               ^ d: src/main/java/ladle/Cup.java:5
    void g(List<Cup> more) { more.sort((x, y) -> x.size() - y.size()); }
    //                                                        ^ d: src/main/java/ladle/Cup.java:5
    void h() { loose.forEach(x -> x.fill()); }
    //                              ^ d: !jump
    void i() { c.fill(); }
    //           ^ d: src/main/java/ladle/Mug.java:4
}
