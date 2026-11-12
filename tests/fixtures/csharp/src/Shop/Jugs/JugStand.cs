namespace Shop.Jugs;

public class JugStand
{
    private class Pot
    {
        public void Boil() { }
    }

    public void Run()
    {
        Jug.Builder().Size(2).Build().Boil();
        //  ^ d: src/Shop/Jugs/Jug.cs:7
        //            ^ d: src/Shop/Jugs/Jug.cs:12
        //                    ^ d: src/Shop/Jugs/Jug.cs:14
        //                            ^ d: src/Shop/Jugs/Jug.cs:5
        Jug.Builder()
            .Size(2)
            //^ d: src/Shop/Jugs/Jug.cs:12
            .Build()
            //^ d: src/Shop/Jugs/Jug.cs:14
            .Boil();
            //^ d: src/Shop/Jugs/Jug.cs:5
        Mystery.Builder().Size(2).Boil();
        //                ^ d: !jump
        //                        ^ d: !jump
    }
}

public class JugShelf
{
    public void Shadowed(System.Func<object> make)
    {
        var Jug = make();
        Jug.Builder().Size(2);
        //            ^ d: !jump
    }
}
