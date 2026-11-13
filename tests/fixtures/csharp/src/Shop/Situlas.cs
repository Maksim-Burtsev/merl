using System;

namespace Shop.Situlas
{
    public class Situla
    {
        public void Tipple() { }
    }

    public interface ITippler
    {
        void Tipple();
    }

    [Obsolete("old")] public class Pail<Situla>
    {
        public Situla Held;
        //     ^ d: src/Shop/Situlas.cs:15
        public void Use() { Held.Tipple(); }
        //                       ^ d: picker src/Shop/Situlas.cs:7, src/Shop/Situlas.cs:12; want none (#1438)
    }

    public class Pails
    {
        [Obsolete("old")] public void Tip<Situla>(Situla x) { x.Tipple(); }
        //                                        ^ d: src/Shop/Situlas.cs:25
        //                                                      ^ d: none
        public void Pair<T, Situla>(Situla x) where T : class { x.Tipple(); }
        //                          ^ d: src/Shop/Situlas.cs:28
        //                                                        ^ d: none
        public void Plain(Situla x) { x.Tipple(); }
        //                ^ d: src/Shop/Situlas.cs:5
        //                              ^ d: src/Shop/Situlas.cs:7
        public delegate Situla Pour<Situla>(Situla x);
        //                                  ^ d: src/Shop/Situlas.cs:34
        public Situla Make<Situla>() where Situla : new() => new Situla();
        //     ^ d: src/Shop/Situlas.cs:36
        //                                                       ^ d: src/Shop/Situlas.cs:36
        public void Outer()
        {
            Situla Local<Situla>(Situla x) => x;
          //^ d: src/Shop/Situlas.cs:41
          //                     ^ d: src/Shop/Situlas.cs:41
            Local(new Situla());
        }
        public System.Collections.Generic.List<Situla> Many<Situla>(Situla x) => null;
        //                                     ^ d: src/Shop/Situlas.cs:46
        //                                                          ^ d: src/Shop/Situlas.cs:46
    }

    public record Cask<Situla>(Situla Item);
    //                         ^ d: src/Shop/Situlas.cs:51
    public readonly struct Tun<Situla>
    {
        public Tun(Situla x) { x.Tipple(); }
        //         ^ d: src/Shop/Situlas.cs:53
        //                       ^ d: none
    }
    public interface IGet
    {
        Situla Get<Situla>();
      //^ d: src/Shop/Situlas.cs:61
    }
    public class Keg<Situla> where Situla : ITippler
    {
        public void Use(Situla x) { x.Tipple(); }
        //              ^ d: src/Shop/Situlas.cs:64
        //                            ^ d: picker src/Shop/Situlas.cs:7, src/Shop/Situlas.cs:12
    }
    public class Outer<T>
    {
        public void M<Situla>(Situla x) { x.Tipple(); }
        //                    ^ d: src/Shop/Situlas.cs:72
        //                                  ^ d: none
        public T Kept;
    }
    [Serializable]
    [Obsolete("old", false)]
    public class Vat<Situla>
    {
        public Situla Held;
        //     ^ d: src/Shop/Situlas.cs:79
        public void Use(Situla x) { x.Tipple(); }
        //              ^ d: src/Shop/Situlas.cs:79
        //                            ^ d: none
    }
}
