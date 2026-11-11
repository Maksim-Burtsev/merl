using static Shop.Rookery.Chessman;

namespace Shop.Rookery;

// A case a using static enum brings in gives way to a member or a parameter of the same name (#1448).
public class Shade
{
    int Rook => 3;

    public int Sit() => Rook;
                     // ^ d: src/Shop/Rookery/Shade.cs:8

    public int Take(int Rook) => Rook;
                              // ^ d: src/Shop/Rookery/Shade.cs:13

    public Chessman Lone() => Pawn;
                           // ^ d: src/Shop/Rookery/Chessman.cs:6

    public object Named(object o)
    {
        var t = (Knight: 1, Pawn: 2);
                         // ^ d: none
        var n = new { Pawn = 1 };
                   // ^ d: none
        var b = o is System.Version { Pawn: > 0 };
                                   // ^ d: none
        return (Pawn: 1, t);
             // ^ d: none
    }
}
