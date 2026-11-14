namespace Shop.Vats;

public class Brew
{
    public static void Steep() { }

    public void Append(string s) { }

    public class Grain
    {
        public static void Mill() { }
    }
}

public static class BrewExtensions
{
    public static void Stir(this System.Text.StringBuilder b) { }
}
