namespace Shop.Tea;

public partial class Kettle
{
    private int Brew() => spout * 2;
    //                    ^ d: src/Shop/Tea/Kettle.cs:15

    public string Recipe() => @"
public int Brew() => 0;
class Kettle {
" + """
    public int Grind() => 1;
    public class Teapot
    """;

    public int Again() => Brew() + Grind() + Rinse();
    //                    ^ d: src/Shop/Tea/Kettle.Parts.cs:5
    //                             ^ d: src/Shop/Tea/Kettle.cs:40
    //                                       ^ d: src/Shop/Tea/Kettle.cs:42
}
