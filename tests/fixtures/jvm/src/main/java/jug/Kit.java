package jug;

class Growler {
    Growler make() {
        return this;
    }

    void quaff() {
    }
}

class Jug {
    static Jug make() {
        return new Jug();
    }

    void quaff() {
    }
}

class Tureen {
    static Tureen make() {
        return new Tureen();
    }

    void quaff() {
    }
}

class Carboy {
    static Carboy make() {
        return new Carboy();
    }

    void quaff() {
    }
}
