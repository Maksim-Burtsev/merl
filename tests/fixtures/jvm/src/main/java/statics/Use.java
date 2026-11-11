package statics;

public class Use {
    void go(Cup cup) {
        Cup.make();
//          ^ d: src/main/java/statics/Cup.java:5
        statics.Cup.make();
//                  ^ d: picker src/main/java/sluice/Weir.java:37, src/main/java/statics/Cup.java:5, src/main/java/statics/Mug.java:5, src/main/kotlin/gilded/Ledge.kt:11, src/main/kotlin/sluice/Dam.kt:35, src/main/scala/sluice/Lock.scala:21, src/main/scala/tureens/Outside.scala:32; want src/main/java/statics/Cup.java:5 (#1489)
        Mug.make();
//          ^ d: src/main/java/statics/Mug.java:5
        cup.make(2);
//          ^ d: src/main/java/statics/Cup.java:8
        Cup.make().fill();
//                 ^ d: src/main/java/statics/Cup.java:11
        Mug.make().fill();
//                 ^ d: src/main/java/statics/Mug.java:8
        Cup.Size.LARGE.grow();
//                     ^ d: picker src/main/java/pins/Pins.java:35, src/main/java/pins/Pins.java:43, src/main/java/statics/Cup.java:15, src/main/java/statics/Mug.java:12, src/main/kotlin/pins/Sprout.kt:5, src/main/kotlin/pins/Sprout.kt:14, src/main/kotlin/winch/Winch.kt:13; want src/main/java/statics/Cup.java:15 (#1489)
        Mug.Size.LARGE.grow();
//                     ^ d: picker src/main/java/pins/Pins.java:35, src/main/java/pins/Pins.java:43, src/main/java/statics/Cup.java:15, src/main/java/statics/Mug.java:12, src/main/kotlin/pins/Sprout.kt:5, src/main/kotlin/pins/Sprout.kt:14, src/main/kotlin/winch/Winch.kt:13; want src/main/java/statics/Mug.java:12 (#1489)
    }
}
