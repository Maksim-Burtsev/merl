using Grams = System.Int32;

namespace Shop.Rules;

// A positional record's parameter is a member of the record.
public record Spot(int Lat, int Lon);

public class Mapper
{
    public int North(Spot spot) => spot.Lat;
    //                                  ^ d: src/Shop/Rules/Rules.cs:6
}

// `base` is the first type the header names.
public class Vault
{
    public virtual void Seal() { }
}

public class IronVault : Vault
{
    public override void Seal()
    {
        base.Seal();
        //   ^ d: src/Shop/Rules/Rules.cs:17
    }
}

// A static method of a type the project declares gives its return type.
public class Parcel
{
    public void Frank() { }
}

public class Envelope
{
    public void Frank() { }
}

public static class ParcelFactory
{
    public static Parcel Make() => new Parcel();
}

public class PostOffice
{
    public void Send()
    {
        var parcel = ParcelFactory.Make();
        parcel.Frank();
        //     ^ d: src/Shop/Rules/Rules.cs:32
    }
}

// A local of another method is no member: the walk goes on to the base.
public class TillBase
{
    protected int tally;
}

public class Till : TillBase
{
    void Tot()
    {
        var tally = 1;
        Print(tally);
    }

    int Read() => tally;
    //            ^ d: src/Shop/Rules/Rules.cs:58
}

// A local is seen from its own method alone, and never behind a dot.
public class Weigher
{
    void Load(dynamic box)
    {
        var heft = 1;
        Print(box.heft);
        //        ^ d: none
    }

    void Show()
    {
        Print(heft);
        //    ^ d: none
    }
}

// The receiver of a call passes an extension method's `this`.
public static class WeigherExtensions
{
    public static int Calibrate(this Weigher weigher, int by) => by;
}

public class Dial
{
    public int Calibrate(int a, int b) => a + b;
}

public class Workshop
{
    public void Tune(dynamic gauge)
    {
        gauge.Calibrate(2);
        //    ^ d: src/Shop/Rules/Rules.cs:93
    }
}

// A type the project declares only as an alias is no type the rules read: the name decides.
public class Jelly
{
    public int Wobble() => 0;
}

public class Hopper
{
    Grams load;

    int Peek() => load.Wobble();
    //                 ^ d: src/Shop/Rules/Rules.cs:113
}

// An extension method the project declares for another type may still be it: the name decides.
public static class SeqExtensions
{
    public static int Spread(this IEnumerable<int> xs) => 0;
}

public class Counter
{
    public int Sum()
    {
        List<int> nums = new();
        return nums.Spread();
        //          ^ d: src/Shop/Rules/Rules.cs:127
    }
}

// A base declared twice proves nothing: the name decides.
public class Twin
{
    public void Nudge() { }
}

public class Kid : Twin, IDisposable
{
    public void Dispose() { }
}

public class Nursery
{
    public void Rock(Kid kid)
    {
        kid.Nudge();
        //  ^ d: picker src/Shop/Rules/Rules.cs:143, src/Shop/Rules/Old/Twin.cs:5
    }
}

// An initializer sets a field or a property, never an extension method.
public static class BuilderExtensions
{
    public static void Capacity(this StringBuilder sb) { }
}

public class Printer
{
    public StringBuilder Begin() => new StringBuilder { Capacity = 4 };
    //                                                  ^ d: none
}
