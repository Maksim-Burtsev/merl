namespace Shop.Tea;

public class Urn
{
    public int Drain() => 1;
}

public class Samovar : Urn
{
    private new int Drain() => 2;
}

public class Sink
{
    public int Drain() => 3;
}
