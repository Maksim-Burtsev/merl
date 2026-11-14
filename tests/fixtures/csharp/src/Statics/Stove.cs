namespace Statics;

public class Stove
{
    public void Light(Samovar k)
    {
        Samovar.Boil();
//              ^ d: src/Statics/Samovar.cs:5
        k.Boil(2);
//        ^ d: src/Statics/Samovar.cs:9
        Samovar.Boil().Whistle();
//                     ^ d: src/Statics/Samovar.cs:13
        Samovar.Shared.Whistle();
//                     ^ d: src/Statics/Samovar.cs:13
        Samovar.Reserve.Whistle();
//                      ^ d: !jump
        Kettle.Shared.Whistle();
//                    ^ d: src/Statics/Kettle.cs:7
    }
}
