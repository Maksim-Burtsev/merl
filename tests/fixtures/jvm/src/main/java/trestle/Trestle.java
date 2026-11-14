package trestle;

public class Trestle<T> {
    T item;
    public T crown() { return item; }
    public <V> V grab(V v) { return v; }
}
