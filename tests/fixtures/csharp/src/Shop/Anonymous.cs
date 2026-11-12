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
    }
}
