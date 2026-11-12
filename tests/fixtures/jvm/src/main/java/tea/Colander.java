package tea;

@interface Sieved {
    int gauge();
    String pekoe() default "";
}

@Sieved(gauge = 5, pekoe = "fine")
//^ d: src/main/java/tea/Colander.java:3
//      ^ d: src/main/java/tea/Colander.java:4
//                 ^ d: src/main/java/tea/Colander.java:5
@Deprecated
//^ d: none
public class Colander {
    @Override
    // ^ d: none
    public int hashCode() {
        return gauge();
    }

    @SuppressWarnings("unused")
    // ^ d: none
    int gauge() {
        return 1;
    }
}
