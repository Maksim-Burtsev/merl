package calls;

class Evict {
    public boolean evict(String c, int limit) {
        return true;
    }

    public boolean evict(String c) {
        return evict(c, 20);
        //     ^ d: src/main/java/calls/Evict.java:4
    }
}
