package locals.b;

class Invoice {
    int sum(Line line) {
        return line.total;
        //          ^ d: none
    }
}
