using Brew = System.Text.StringBuilder;
using Enc = System.Text.Encoding;

namespace Shop.Jugs.Foreign;

public class Tun
{
    public void Run()
    {
        Brew.Steep();
        //   ^ d: none
        var b = new Brew();
        //          ^ d: !jump
        b.Append("x");
        //^ d: none
        b.Stir();
        //^ d: src/Shop/Jugs/Foreign/Brew.cs:17
        new Brew().Append("x");
        //         ^ d: none
        Brew x = null;
        x.Append("y");
        //^ d: none
        Brew.Grain.Mill();
        //   ^ d: none
        //         ^ d: none
        Enc.UTF8.Brine();
        //       ^ d: src/Shop/Jugs/Foreign/Brew.cs:18
    }
}
