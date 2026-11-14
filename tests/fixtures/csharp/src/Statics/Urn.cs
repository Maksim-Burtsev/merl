using Pot = Statics.Samovar;

namespace Statics;

public class Urn
{
    public void Pour()
    {
        Samovar.Size.Large.Grow();
//                         ^ d: src/Statics/Samovar.cs:21
        Kettle.Size.Large.Grow();
//                        ^ d: src/Statics/Kettle.cs:13
        Samovar.Size.Large;
//                   ^ d: src/Statics/Samovar.cs:20
        Samovar.Size.Large.Next.Grow();
//                              ^ d: src/Statics/Samovar.cs:21
        Pot.Boil();
//          ^ d: src/Statics/Samovar.cs:5
        Pot.Boil().Whistle();
//                 ^ d: src/Statics/Samovar.cs:13
    }
}
