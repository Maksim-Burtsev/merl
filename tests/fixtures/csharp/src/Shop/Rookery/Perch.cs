using static Shop.Rookery.Corvid;

namespace Shop.Rookery;

// A case two enums of the project share, written without its enum (#1448).
public static class Perch
{
    public static int Sit(Corvid bird)
    {
        switch (bird)
        {
            case Rook:
              // ^ d: none; want src/Shop/Rookery/Corvid.cs:6 (#1448)
                return 1;
            case Corvid.Raven:
                return 0;
        }
        return 0;
    }

    public static int Advance(Chessman piece) => piece switch
    {
        Chessman.Rook => 1,
              // ^ d: src/Shop/Rookery/Chessman.cs:5
        _ => 0,
    };

    public static Corvid Plain() => Rook;
                                 // ^ d: none; want src/Shop/Rookery/Corvid.cs:6 (#1448)
}
