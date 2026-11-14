namespace Statics;

public class Range
{
    public Samovar Samovar { get; } = new Samovar();

    public void Heat()
    {
        Samovar.Boil(2);
//              ^ d: src/Statics/Samovar.cs:9
    }
}
