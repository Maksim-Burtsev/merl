namespace Statics;

public class Samovar
{
    public static Samovar Boil()
    {
        return new Samovar();
    }
    public Samovar Boil(int n)
    {
        return this;
    }
    public void Whistle()
    {
    }
    public static Samovar Shared { get; } = new Samovar();
    public Samovar Reserve { get; } = new Samovar();
}
