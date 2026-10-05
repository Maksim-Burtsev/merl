package abacus;

public class Till {
    public static int unlock() {
        return 1;
    }

    int tallied(Abacus a) {
    //          ^ d: src/main/groovy/abacus/Abacus.groovy:21
        return a.reckon().intValue();
        //       ^ d: src/main/groovy/abacus/Abacus.groovy:34
    }
}
