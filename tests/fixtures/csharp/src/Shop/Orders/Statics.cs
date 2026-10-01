using static Shop.Pricing.Prices;

namespace Shop.Orders;

public class Register
{
    // A `using static` brings `Prices`'s members in: another type's member stays a candidate.
    public int Sum() => Discount(5);
    //                  ^ d: src/Shop/Pricing/Pricing.cs:46
}

public class DupBase { }

public class UsesDup : DupBase
{
    // `DupBase` is declared twice: what it declares is not known, so nothing is dropped.
    public int Go() => Spare();
    //                 ^ d: src/Shop/Orders/Statics.cs:23
}

public class SpareHolder
{
    public int Spare() => 1;
}

public class Scale
{
    public int Weigh(int grams) => grams;
    public int Weigh(int grams, int tare) => grams - tare;
}

public class Weighing
{
    public int Net()
    {
        var scale = new Scale();
        return scale.Weigh(1, 2) + scale.Weigh(1, 2, 3);
        //           ^ d: src/Shop/Orders/Statics.cs:29
        //                               ^ d: picker src/Shop/Orders/Statics.cs:28, src/Shop/Orders/Statics.cs:29
    }
}

public static class Twice
{
    public static int Of(this int x) => x;
    public static int Of(this int x, int by) => x * by;
}

public class Doubler
{
    // Called on its own class, the extension takes its `this` as the first argument.
    public int Run() => Shop.Orders.Twice.Of(3);
    //                                    ^ d: src/Shop/Orders/Statics.cs:45
}

public class Helpers
{
    public static int Helper() => 1;

    public class Inner : HelperBase
    {
        // A base's private member is out of reach: the type around it answers.
        public int Use() => Helper();
        //                  ^ d: src/Shop/Orders/Statics.cs:58
    }
}

public class HelperBase
{
    private int Helper() => 2;
}

public class LogBase
{
    public void Log(int a, int b) { }
}

public class Logger : LogBase
{
    public void Log(int a) { }

    // The type's own overload cannot take two arguments: its base's can.
    public void Run() => Log(1, 2);
    //                   ^ d: src/Shop/Orders/Statics.cs:75
}

public enum Phase { Open, Shut }

public class Stage
{
    public Phase Phase { get; set; }

    // "Color Color": the property or its type, both offered.
    public bool Done() => Phase == Phase.Open;
    //                             ^ d: picker src/Shop/Orders/Statics.cs:87, src/Shop/Orders/Statics.cs:91

    public partial class Page : PageBase
    {
        public void Go() => Show();
        //                  ^ d: src/Shop/Orders/Statics.cs:106
    }
}

public class PageBase
{
    public void Show() { }
}
