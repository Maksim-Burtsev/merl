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
//                     ^ d: picker src/Statics/Kettle.cs:7, src/Statics/Samovar.cs:13; want src/Statics/Samovar.cs:13 (#1489)
        Samovar.Reserve.Whistle();
//                      ^ d: !jump
        Kettle.Shared.Whistle();
//                    ^ d: picker src/Statics/Kettle.cs:7, src/Statics/Samovar.cs:13; want src/Statics/Kettle.cs:7 (#1489)
    }
}
