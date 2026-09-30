package refs;

import java.util.List;

class Use {
    void names(List<Inner> items) {
        items.stream().map(Inner::getName);
        //                        ^ d: src/main/java/refs/Use.java:13
        // status: getName → Use.Inner.getName (via Inner)
    }

    static class Inner {
        String getName() {
            return "";
        }
    }

    static class Other {
        String getName() {
            return "";
        }
    }
}
