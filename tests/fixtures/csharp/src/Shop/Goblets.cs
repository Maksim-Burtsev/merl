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
            //^ d: none
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

    public class Shelf<Goblet>
    {
        private Goblet held;
        public Shop.Goblets.Goblet cup;
        public void Sip(Goblet x)
        {
            x.Quaff();
            //^ d: none
            held.Quaff();
            //   ^ d: none
            this.held.Quaff();
            //        ^ d: none
            cup.Quaff();
            //  ^ d: src/Shop/Goblets.cs:5
        }
    }

    public class Cellar<T> where T : Goblet
    {
        private T held;
        public void Sip(T x)
        {
            x.Quaff();
            //^ d: src/Shop/Goblets.cs:5
            held.Quaff();
            //   ^ d: src/Shop/Goblets.cs:5
        }
    }

    public interface ISip
    {
        void Sip<Goblet>(Goblet x)
        {
            x.Quaff();
            //^ d: none
        }
    }

    public class GobletTaste
    {
        public void Plain(Goblet x)
        {
            x.Quaff();
            //^ d: src/Shop/Goblets.cs:5
        }
        public void Concrete<T>(Goblet x, T y)
        {
            x.Quaff();
            //^ d: src/Shop/Goblets.cs:5
            y.Quaff();
            //^ d: none
        }
        public static Dictionary<string, T> Pair<K, T>(T x)
        {
            x.Quaff();
            //^ d: none
            return null;
        }
        public void Below<T>(T x)
            where T : class, Goblet, new()
        {
            x.Quaff();
            //^ d: src/Shop/Goblets.cs:5
        }
        public void Wrapped<T>(
            T x) where T : Goblet
        {
            x.Quaff();
            //^ d: src/Shop/Goblets.cs:5
        }
        public void Called(Goblet x)
        {
            Run<Goblet>(x, y =>
            {
                y.Quaff();
                //^ d: src/Shop/Goblets.cs:5
            });
            return Deserialize<Goblet>(s, z =>
            {
                z.Quaff();
                //^ d: src/Shop/Goblets.cs:5
            });
        }
    }

    public class Flask<Goblet>
    {
        public Goblet Held { get; set; }
        public void Pour(Goblet x) => x.Quaff();
        //                              ^ d: none
        public void Keep(Goblet x)
        {
            Goblet y = x;
            y.Quaff();
            //^ d: none
            Held.Quaff();
            //   ^ d: none
        }
    }

    public class Ewer<Goblet>
    {
        private Goblet made = Make();
        public Goblet Kept { get => Make(); }
        public Shop.Goblets.Goblet Cup { get; set; }
        private Shop.Goblets.Goblet cup = new Shop.Goblets.Goblet();
        public void Keep()
        {
            made.Quaff();
            //   ^ d: none
            Kept.Quaff();
            //   ^ d: none
            Cup.Quaff();
            //  ^ d: src/Shop/Goblets.cs:5
            cup.Quaff();
            //  ^ d: src/Shop/Goblets.cs:5
        }
    }

    public interface IPitcher<in Goblet>
    {
        void Tip(Goblet x)
        {
            x.Quaff();
            //^ d: none
        }
    }

    public interface IFunnel<out Goblet>
    {
        Goblet Make();
        void Tip()
        {
            Goblet y = Make();
            y.Quaff();
            //^ d: none
        }
    }

    public interface IBounded<in T> where T : Goblet
    {
        void Tip(T x)
        {
            x.Quaff();
            //^ d: src/Shop/Goblets.cs:5
        }
    }

    public abstract class Decanter
    {
        public abstract void Tip<T>(T x) where T : Goblet;
    }

    public interface ITip
    {
        void Tip<T>(T x) where T : Goblet;
    }

    public class Carafe : Decanter, ITip
    {
        public override void Tip<T>(T x)
        {
            x.Quaff();
            //^ d: src/Shop/Goblets.cs:5
        }
        void ITip.Tip<T>(T x)
        {
            x.Quaff();
            //^ d: src/Shop/Goblets.cs:5
        }
    }

    public class Jug<T> : ITip
    {
        void ITip.Tip<T>(T x)
        {
            x.Quaff();
            //^ d: src/Shop/Goblets.cs:5
        }
    }
}
