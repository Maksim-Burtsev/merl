using Flagon = Shop.Renamed.Cups.Pitchel;

namespace Shop.Renamed.Decoy
{
    public static class Hidden
    {
        public static int Run() => new Flagon().Tilt();
        //                             ^ d: picker src/Shop/Renamed/Hidden.cs:1, src/Shop/Renamed/Flagon.cs:3, src/Shop/Renamed/Poured.cs:1
        //                                      ^ d: picker src/Shop/Renamed/Flagon.cs:5, src/Shop/Renamed/OtherPitchel.cs:5, src/Shop/Renamed/Pitchel.cs:5
    }
}
