package scopes.b;

public class Prefs {
    private final Scheduler scheduler = null;

    private static String parentNameOf(String item) {
        return item;
    }

    String copy(Prefs other) {
        return other.parentNameOf("x");
        //           ^ d: picker src/main/java/scopes/b/Prefs.java:6
        // status: parentNameOf: by name, 1 match
    }
}
