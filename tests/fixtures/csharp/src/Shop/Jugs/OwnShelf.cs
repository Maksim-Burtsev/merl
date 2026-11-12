namespace Shop.Jugs.Own;

public class SortedDictionary<K, V>
{
    public Pan this[K key] => new Pan();
}

public class Pan
{
    public void Boil() { }
}

public class OwnShelf
{
    private readonly SortedDictionary<string, Shop.Jugs.Jug> byName = new();

    public void Heat()
    {
        byName["a"].Boil();
        //          ^ d: !jump
    }
}
