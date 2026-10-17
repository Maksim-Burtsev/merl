using System;
using Stale = Shop.Tea.Stale;

namespace Shop.Tea;

public interface IPourer
{
    int Pour();
}

public class Vessel
{
    public virtual int Rinse() => 1;
}

public class Teapot
{
    public int Pour(int cups) => cups;

    public int Lid { get; set; }
}

public static class Spices
{
    public static int Grind() => 3;

    public static int Steam() => 4;
}

public class Table
{
    public int Set(Teapot pot, Stale.Kettle old)
    {
        var kettle = new Kettle(1);
        //               ^ d: src/Shop/Tea/Kettle.cs:17
        Kettle near = kettle;
        return kettle.Pour(1) + pot.Pour(1) + old.Pour(1) + near.Pour(1) + Spices.Grind();
        //            ^ d: src/Shop/Tea/Kettle.cs:24
        //                          ^ d: src/Shop/Tea/Teapot.cs:18
        //                                        ^ d: src/Shop/Tea/Stale/Kettle.cs:5
        //                                                       ^ d: src/Shop/Tea/Kettle.cs:24
        //                                                                        ^ d: src/Shop/Tea/Teapot.cs:25
    }

    public int Lid(Teapot pot)
    {
        int Lid = pot.Lid;
        //            ^ d: src/Shop/Tea/Teapot.cs:20
        return Lid;
        //     ^ d: src/Shop/Tea/Teapot.cs:47
    }

    public int Shadow()
    {
        var cup = new Kettle(2);
        Func<Teapot, int> f = cup => cup.Pour(1);
        //                               ^ d: src/Shop/Tea/Teapot.cs:18
        return f(new Teapot()) + cup.Pour(1);
        //                           ^ d: src/Shop/Tea/Kettle.cs:24
    }

    public int Fresh(Stale.Kettle stale) => stale.Pour(3) + new Stale.Kettle().Pour(4);
    //                     ^ d: src/Shop/Tea/Stale/Kettle.cs:3
    //                                            ^ d: src/Shop/Tea/Stale/Kettle.cs:5
    //                                                                         ^ d: src/Shop/Tea/Stale/Kettle.cs:5

    public int Names(IPourer pourer) => pourer.Pour() + nameof(Steam).Length;
    //                                         ^ d: src/Shop/Tea/Teapot.cs:8

    public int Steam() => Spices.Steam();
    //                           ^ d: src/Shop/Tea/Teapot.cs:27
}

public class Tray
{
    public Teapot Cosy() => new Teapot();
    //     ^ d: src/Shop/Tea/Teapot.cs:16
}
