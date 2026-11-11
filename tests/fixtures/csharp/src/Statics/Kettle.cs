namespace Statics;

public class Kettle
{
    public static Kettle Shared { get; } = new Kettle();

    public void Whistle()
    {
    }
}
