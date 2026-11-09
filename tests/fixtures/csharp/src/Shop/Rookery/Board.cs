using static Shop.Rookery.Chessman;

namespace Shop.Rookery;

public static class Board
{
    public static int Advance(Chessman piece) => piece switch
    {
        Rook => 1,
     // ^ d: none; want src/Shop/Rookery/Chessman.cs:5 (#1448)
        _ => 0,
    };
}
