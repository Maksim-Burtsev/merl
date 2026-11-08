using System;
using System.Linq;
using static Shop.Rookery.Chessman;

namespace Shop.Rookery;

public static class Arms
{
    public static int Inline(Chessman piece) => piece switch { Pawn => 0, Rook => 1, _ => 2 };
                                                                       // ^ d: none; want src/Shop/Rookery/Chessman.cs:5 (#1448)
    public static int Either(Chessman piece) => piece switch
    {
        Pawn or Rook => 1,
             // ^ d: none; want src/Shop/Rookery/Chessman.cs:5 (#1448)
        _ => 0,
    };

    public static int Typed(object o) => o switch
    {
        Chessman c => (int)c,
                        // ^ d: src/Shop/Rookery/Arms.cs:20
        _ => 0,
    };

    public static int Body(int[] all, Chessman piece) => piece switch
    {
        Rook => all.Count(Rook => Rook > 0),
                               // ^ d: src/Shop/Rookery/Arms.cs:27
        _ => 0,
    };

    public static Func<int, int> Lifted() => Call(
        Rook => Rook + 1);
             // ^ d: src/Shop/Rookery/Arms.cs:33

    static Func<int, int> Call(Func<int, int> f) => f;
}
