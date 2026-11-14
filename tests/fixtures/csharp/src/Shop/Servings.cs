namespace Shop.Servings;

public class Crock
{
    public void Stew()
    {
    }
}

public class Wok
{
    public void Stew()
    {
    }
}

public class Hamper
{
    public void Deconstruct(out Crock crock, out Wok wok)
    {
        crock = new Crock();
        wok = new Wok();
    }
}

public static class Serving
{
    public static (Crock, Wok) ServePair() => (new Crock(), new Wok());

    public static (Crock Crock, Wok Wok) ServeNamed() => (new Crock(), new Wok());

    public static void Unpack()
    {
        var (c, w) = ServePair();
        w.Stew();
//        ^ d: src/Shop/Servings.cs:12
        c.Stew();
//        ^ d: src/Shop/Servings.cs:5
        (Crock nc, Wok nw) = ServeNamed();
        nw.Stew();
//         ^ d: src/Shop/Servings.cs:12
        var (kc, kw) = new Hamper();
        kw.Stew();
//         ^ d: src/Shop/Servings.cs:12
        kc.Stew();
//         ^ d: src/Shop/Servings.cs:5
        var (mc, mw, mx) = new Hamper();
        mw.Stew();
//         ^ d: picker src/Shop/Servings.cs:5, src/Shop/Servings.cs:12
    }
}
