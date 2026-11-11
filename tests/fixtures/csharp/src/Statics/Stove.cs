namespace Statics;

public class Stove
{
    public void Light(Kettle k)
    {
        Kettle.Boil();
//             ^ d: src/Statics/Kettle.cs:5
        k.Boil(2);
//        ^ d: src/Statics/Kettle.cs:9
        Kettle.Boil().Whistle();
//                    ^ d: src/Statics/Kettle.cs:13
    }
}
