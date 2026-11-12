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

public class Kit
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
//        ^ d: picker src/Shop/Servings.cs:5, src/Shop/Servings.cs:12; want src/Shop/Servings.cs:12 (#1484)
        c.Stew();
//        ^ d: picker src/Shop/Servings.cs:5, src/Shop/Servings.cs:12; want src/Shop/Servings.cs:5 (#1484)
        (Crock nc, Wok nw) = ServeNamed();
        nw.Stew();
//         ^ d: src/Shop/Servings.cs:12
        var (kc, kw) = new Kit();
        kw.Stew();
//         ^ d: picker src/Shop/Servings.cs:5, src/Shop/Servings.cs:12; want src/Shop/Servings.cs:12 (#1484)
    }
}
