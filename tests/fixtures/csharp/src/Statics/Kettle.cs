namespace Statics;

public class Kettle
{
    public static Kettle Boil()
    {
        return new Kettle();
    }
    public Kettle Boil(int n)
    {
        return this;
    }
    public void Whistle()
    {
    }
}
