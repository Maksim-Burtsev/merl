using Flagon = Shop.Renamed.Cups.Pitchel;

namespace Shop.Renamed
{
    public static class Poured
    {
        public static int Run()
        {
            var a = new Flagon();
            //          ^ d: src/Shop/Renamed/Pitchel.cs:3
            return a.Tilt() + new Flagon().Tilt();
            //       ^ d: src/Shop/Renamed/Pitchel.cs:5
            //                             ^ d: src/Shop/Renamed/Pitchel.cs:5
        }
    }
}
