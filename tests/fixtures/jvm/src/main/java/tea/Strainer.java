package tea;

import java.util.function.IntUnaryOperator;

public class Strainer {
    private int mesh = 1;

    public int quoted() {
        String s = "f(mesh -> 1)"; int k = mesh;
        //                                 ^ d: src/main/java/tea/Strainer.java:6
        return k;
    }

    public int closed(java.util.List<Integer> xs) {
        xs.forEach(mesh -> { }); int k = mesh;
        //                               ^ d: src/main/java/tea/Strainer.java:6
        return k;
    }

    public int assigned() {
        IntUnaryOperator f = mesh -> mesh; int k = mesh;
        //                                         ^ d: src/main/java/tea/Strainer.java:6
        //                           ^ d: src/main/java/tea/Strainer.java:21
        return k + f.applyAsInt(1);
    }

    public int paired() {
        int k = g("a,mesh -> b", mesh);
        //                       ^ d: src/main/java/tea/Strainer.java:6
        return k;
    }

    private int g(String a, int b) { return b; }
}
