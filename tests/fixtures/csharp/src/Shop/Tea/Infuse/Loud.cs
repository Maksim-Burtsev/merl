using System.Text;

namespace Shop.Tea.Infuse;

public static class Loud
{
    public static StringBuilder Shout(
        this StringBuilder sb) => sb;
}

public class Yell
{
    public int Chained(string name)
    {
        var sb = new StringBuilder();
        sb.Append(name).Shout();
        //              ^ d: src/Shop/Tea/Infuse/Loud.cs:7
        var told = new StringBuilder().Append(name);
        told.Shout();
        //   ^ d: src/Shop/Tea/Infuse/Loud.cs:7
        sb.Shout();
        //  ^ d: src/Shop/Tea/Infuse/Loud.cs:7
        return sb.Length;
    }
}
