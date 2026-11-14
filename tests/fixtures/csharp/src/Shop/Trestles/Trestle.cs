namespace Shop.Trestles
{
    public class Trestle<T>
    {
        public T Item;
        public T Crown() { return Item; }
        public V Grab<V>(V v) { return v; }
    }
}
