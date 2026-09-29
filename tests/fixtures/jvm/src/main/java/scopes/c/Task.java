package scopes.c;

abstract class Task {
    protected Task(String source) {}

    abstract String f(String s);

    void retry(String part) {}
}
