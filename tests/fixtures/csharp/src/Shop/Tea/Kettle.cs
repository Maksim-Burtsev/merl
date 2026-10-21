using static Shop.Tea.Spices;

namespace Shop.Tea;

/*
public class Teapot
public int Pour(int cups)
*/

/// <summary>
/// public int Brew() => 0;
/// </summary>
public partial class Kettle : Vessel, IPourer
{
    private int spout;

    public Kettle(int spout)
    {
        this.spout = spout;
        //   ^ d: src/Shop/Tea/Kettle.cs:15
        //           ^ d: src/Shop/Tea/Kettle.cs:17
    }

    public int Pour(int cups) => cups + spout;

    public int Pour(string cups) => cups.Length;

    public int Serve()
    {
        int spout = 1;
        return spout + this.spout + Pour("two") + Pour(2) + Grind() + Brew();
        //     ^ d: src/Shop/Tea/Kettle.cs:30
        //                  ^ d: src/Shop/Tea/Kettle.cs:15
        //                          ^ d: src/Shop/Tea/Kettle.cs:26
        //                                        ^ d: src/Shop/Tea/Kettle.cs:24
        //                                                  ^ d: src/Shop/Tea/Kettle.cs:40
        //                                                            ^ d: src/Shop/Tea/Kettle.Parts.cs:5
    }

    public int Grind() => 7;

    public override int Rinse() => base.Rinse() + Steam();
    //                                  ^ d: src/Shop/Tea/Teapot.cs:13
    //                                            ^ d: src/Shop/Tea/Teapot.cs:27

    int IPourer.Pour() => 0;
}
