package typeuse;

public class Index {
    Object index() {
        return Registry.lookup("spec.name");
//                      ^ d: src/main/java/typeuse/Registry.java:4
    }

    Object code() {
        return Registry.code("spec.name");
//                      ^ d: !jump
    }

    Object single() {
        return Specs.single("spec.name", Index.class);
//                   ^ d: src/main/java/typeuse/Specs.java:4
    }
}
