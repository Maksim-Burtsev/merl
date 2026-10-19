namespace Shop.Tea.Infuse;

public interface ISteeper
{
    int Steep() => 1;

    int Strength { get; }
}

public class Cup
{
    public int Count { get; set; }

    public virtual int Sip() => 1;

    public int Append(string s) => s.Length;

    public int Add(Cup other) => Count + other.Count;

    public Cup Refill() => new Cup();
}

public class Mug : Cup, ISteeper
{
    public override int Sip() => 2;

    public int Strength => 3;
}

public class Brewer
{
    public Brewer Leaf(string name) => this;

    public Brewer Water(int ml) => this;

    public Cup Build() => new Mug();
}

public static class CupExtensions
{
    public static int Warm(this Cup cup) => cup.Count;
}

public class Stove
{
    public int Warm() => 0;

    public int Sip() => 4;
}

public class Glass(int size)
{
    public int Size() => size;
    //                   ^ d: src/Shop/Tea/Infuse/Cup.cs:51

    public int Pour(int size) => size + Size();
    //                           ^ d: src/Shop/Tea/Infuse/Cup.cs:56
    //                                  ^ d: src/Shop/Tea/Infuse/Cup.cs:53
}
