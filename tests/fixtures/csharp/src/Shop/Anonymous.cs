namespace Shop;

class Anonymous
{
    int tally;

    void Go()
    {
        var anon = new { tally = 5, other = 1 };
        System.Console.WriteLine(anon.tally);
//                                    ^ d: src/Shop/Anonymous.cs:9
        var wide = new
        {
            first = 1,
            tally = 2,
        };
        System.Console.WriteLine(wide.tally);
//                                    ^ d: src/Shop/Anonymous.cs:15
        var nest = new
        {
            inner = new { tally = 1 },
            tally = 2,
        };
        System.Console.WriteLine(nest.tally);
//                                    ^ d: src/Shop/Anonymous.cs:22
        var mixed = new
        {
            label = "a, tally, b",
            sum = System.Math.Max(1, tally),
            total = 1 +
                tally,
            tally = 3,
        };
        System.Console.WriteLine(mixed.tally);
//                                     ^ d: src/Shop/Anonymous.cs:32
    }
}
