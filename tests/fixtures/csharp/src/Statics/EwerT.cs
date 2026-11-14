namespace Statics;

public class Ewer<T>
{
    public void Log()
    {
    }

    public void Use(Ewer other)
    {
        other.Log();
//            ^ d: src/Statics/Ewer.cs:7
        Ewer e = Ewer.Ok();
        e.Log();
//        ^ d: src/Statics/Ewer.cs:7
    }
}
