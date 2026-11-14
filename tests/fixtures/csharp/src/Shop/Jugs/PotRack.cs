using Pot = Statics.Samovar;
using Rack = Statics.Samovar;

namespace Shop.Jugs;

public class PotRack
{
    private class Pot
    {
        public static void Boil() { }
    }

    private class Rack
    {
        public static void Boil() { }
    }

    public void Run()
    {
        Pot.Boil();
        //  ^ d: picker src/Shop/Jugs/PotRack.cs:10, src/Shop/Jugs/PotRack.cs:51; want src/Shop/Jugs/PotRack.cs:10 (#1466)
        Rack.Boil();
        //   ^ d: src/Shop/Jugs/PotRack.cs:15
    }
}

public class Shelf
{
    public void Run()
    {
        Pot.Boil();
        //  ^ d: src/Statics/Samovar.cs:5
    }
}

public class Hook : Shelf
{
    public void Hang()
    {
        Pot.Boil();
        //  ^ d: picker src/Shop/Jugs/PotRack.cs:10, src/Shop/Jugs/PotRack.cs:51; want src/Statics/Samovar.cs:5 (#1466)
        Rack.Boil();
        //   ^ d: src/Statics/Samovar.cs:5
    }
}

public class Stove
{
    protected class Pot
    {
        public static void Boil() { }
    }
}

public class Range : Stove
{
    public void Light()
    {
        Pot.Boil();
        //  ^ d: picker src/Shop/Jugs/PotRack.cs:10, src/Shop/Jugs/PotRack.cs:51; want src/Shop/Jugs/PotRack.cs:51 (#1466)
    }
}
