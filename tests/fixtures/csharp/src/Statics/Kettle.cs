namespace Statics;

public class Kettle
{
    public static Kettle Shared { get; } = new Kettle();

    public void Whistle()
    {
    }
    public class Size
    {
        public static readonly Size Large = new Size();
        public void Grow()
        {
        }
    }
}
