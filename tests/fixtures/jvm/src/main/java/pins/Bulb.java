class Bulb {
    void on(Lamp l) {
        l.glow();
        //^ d: src/main/java/pins/Bulb.java:9
    }
}

class Lamp {
    void glow() {}
}
