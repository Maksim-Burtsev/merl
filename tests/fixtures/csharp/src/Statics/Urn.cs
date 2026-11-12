using Pot = Statics.Samovar;

namespace Statics;

public class Urn
{
    public void Pour()
    {
        Samovar.Size.Large.Grow();
//                         ^ d: picker src/Statics/Kettle.cs:13, src/Statics/Samovar.cs:21; want src/Statics/Samovar.cs:21 (#1489)
        Kettle.Size.Large.Grow();
//                        ^ d: picker src/Statics/Kettle.cs:13, src/Statics/Samovar.cs:21; want src/Statics/Kettle.cs:13 (#1489)
        Samovar.Size.Large;
//                   ^ d: src/Statics/Samovar.cs:20
        Pot.Boil();
//          ^ d: src/Shop/Jugs/JugStand.cs:7; want src/Statics/Samovar.cs:5 (#1512)
        Pot.Boil().Whistle();
//                 ^ d: picker src/Statics/Kettle.cs:7, src/Statics/Samovar.cs:13; want src/Statics/Samovar.cs:13 (#1489)
    }
}
