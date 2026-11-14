package shop.pricing;

class Stylus {
    String blot() {
        return "stylus";
    }
}

class Nib {
    String blot() {
        return "nib";
    }
}

public class Quoted {
    String a(Stylus s) {
        return "s.blot()";
        //        ^ d: picker src/main/java/shop/pricing/Quoted.java:4, src/main/java/shop/pricing/Quoted.java:10, …
    }

    String b(Stylus s) {
        return """
            s.blot() \"""
        //    ^ d: picker src/main/java/shop/pricing/Quoted.java:4, src/main/java/shop/pricing/Quoted.java:10, …
            """;
    }

    String c(Stylus s) {
        return "a\\" + s.blot() + '"' + s.blot(); // "
        //               ^ d: src/main/java/shop/pricing/Quoted.java:4
        //                                ^ d: src/main/java/shop/pricing/Quoted.java:4
    }
}
