package jug;

// Each owner declares a field named like a class of Kit.java, and Users.java reaches each one
// from outside: by a subclass, an anonymous subclass and an interface on a header's next line
// (#1514).
class Sideboard {
    protected Growler Jug = new Growler();
}

class Dresser {
    protected Growler Tureen = new Growler();
}

interface Hanger {
    Growler Carboy = new Growler();
}
