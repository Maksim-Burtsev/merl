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
}
