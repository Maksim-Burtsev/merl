using Brew = System.Text.StringBuilder;

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
        //^ d: src/Shop/Jugs/Foreign/Brew.cs:18
        new Brew().Append("x");
        //         ^ d: none
        Brew x = null;
        x.Append("y");
        //^ d: none
        Brew.Grain.Mill();
        //   ^ d: none
        //         ^ d: none
    }
}
