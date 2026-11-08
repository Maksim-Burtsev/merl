namespace Shop.Goblets
{
    public class Goblet
    {
        public void Quaff() { }
    }

    public class Rack<Goblet>
    {
        public Goblet Held;
        //     ^ d: src/Shop/Goblets.cs:8
    }

    public class GobletPour
    {
        public void GobletFill<Goblet>(Goblet x) { }
        //                             ^ d: src/Shop/Goblets.cs:16
        public void GobletWrapped<Goblet>(
            Goblet x)
            //^ d: src/Shop/Goblets.cs:18
        {
        }
        public void GobletPlain(Goblet x) { }
        //                      ^ d: src/Shop/Goblets.cs:3
        public void GobletBound<T>(T x) where T : Goblet { }
        //                                        ^ d: src/Shop/Goblets.cs:3
    }
}
