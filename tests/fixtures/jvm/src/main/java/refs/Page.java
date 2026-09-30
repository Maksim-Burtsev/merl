package refs;

import java.util.List;

class Page {
    void render(List<String> ids) {
        ids.forEach(this::show);
        //                ^ d: src/main/java/refs/Page.java:16
        // status: show → Page.show (via this)
        ids.forEach(id -> this.show(id));
        //                     ^ d: src/main/java/refs/Page.java:16
        ids.forEach(super::equals);
        //                 ^ d: none
    }

    private void show(String id) {}
}
