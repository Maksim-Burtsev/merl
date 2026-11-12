package statics;

public class Use {
    void go(Cup cup) {
        Cup.make();
//          ^ d: src/main/java/statics/Cup.java:5
        statics.Cup.make();
//                  ^ d: src/main/java/statics/Cup.java:5
        Mug.make();
//          ^ d: src/main/java/statics/Mug.java:5
        cup.make(2);
//          ^ d: src/main/java/statics/Cup.java:8
        Cup.make().fill();
//                 ^ d: src/main/java/statics/Cup.java:11
        Mug.make().fill();
//                 ^ d: src/main/java/statics/Mug.java:8
        Cup.Size.LARGE.grow();
//                     ^ d: src/main/java/statics/Cup.java:15
        Mug.Size.LARGE.grow();
//                     ^ d: src/main/java/statics/Mug.java:12
        Cup.SHARED.fill();
//                 ^ d: src/main/java/statics/Cup.java:11
        Cup.Size.LARGE.toString();
//               ^ d: src/main/java/statics/Cup.java:14
        statics.Mug.make();
//                  ^ d: src/main/java/statics/Mug.java:5
        statics.Cup.Size.LARGE.grow();
//                             ^ d: src/main/java/statics/Cup.java:15
    }
}
