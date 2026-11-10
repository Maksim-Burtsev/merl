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
}
