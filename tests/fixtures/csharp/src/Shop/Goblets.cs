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
        public void GobletSip<Goblet>(Goblet x)
        {
            x.Quaff();
            //^ d: src/Shop/Goblets.cs:5; want none (#1438)
        }
        public void GobletSipBound<T>(T x) where T : Goblet
        {
            x.Quaff();
            //^ d: src/Shop/Goblets.cs:5
        }
    }

    public class GobletServe
    {
        public void Serve(Services services)
        {
            services.AddScoped<Goblet>();
            //                 ^ d: src/Shop/Goblets.cs:3
            var all = new List<Goblet>();
            //                 ^ d: src/Shop/Goblets.cs:3
            foreach (var c in Items().OfType<Goblet>())
            //                               ^ d: src/Shop/Goblets.cs:3
            {
                Goblet g = c;
                //^ d: src/Shop/Goblets.cs:3
            }
            store.Update(Arg.Any<Goblet>())
                .Returns(call => call.Arg<Goblet>());
                //                        ^ d: src/Shop/Goblets.cs:3
        }
        public Goblet Load(string s)
        {
            return Deserialize<Goblet>(s);
            //                 ^ d: src/Shop/Goblets.cs:3
        }
    }
}
