using System.Collections.Generic;
using System.Text;
using Brew = Shop.Tea.Infuse.Brewer;

namespace Shop.Tea.Infuse;

/*
public class Cup
public int Sip() => 0;
*/

#if false
public class Mug
{
    public int Count;
}
#endif

public class Tasting
{
    private Stove cup = new Stove();

    public int Outside(string name)
    {
        var sb = new StringBuilder();
        sb.Append(name);
        //  ^ d: none
        sb.Append(name)
            .Append("!");
        //   ^ d: none
        var cups = new List<Cup>();
        cups.Add(new Mug());
        //   ^ d: none
        return cups.Count + sb.Length;
        //          ^ d: none
    }

    public int Chain()
    {
        var leafy = new Brewer()
            .Leaf("green");
        //   ^ d: src/Shop/Tea/Infuse/Cup.cs:32
        var watered = leafy.Leaf("white")
            .Water(80);
        //   ^ d: src/Shop/Tea/Infuse/Cup.cs:34
        var made = new Brewer()
            .Leaf("green")
            .Water(80)
            .Build();
        //   ^ d: src/Shop/Tea/Infuse/Cup.cs:36
        return made.Sip() + new Brew().Leaf("black").Build().Sip();
        //          ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48; want src/Shop/Tea/Infuse/Cup.cs:14 (#1135)
        //                      ^ d: src/Shop/Tea/Infuse/Tasting.cs:3
        //                             ^ d: src/Shop/Tea/Infuse/Cup.cs:32
        //                                                   ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48; want src/Shop/Tea/Infuse/Cup.cs:14 (#1135)
    }

    public int Lines(Brewer brewer)
    {
        return brewer
            .Build()
            .Sip();
        //   ^ d: src/Shop/Tea/Infuse/Cup.cs:14
    }

    public int Loop(List<Mug> mugs)
    {
        int total = 0;
        foreach (var cup in mugs)
        {
            total += cup.Sip();
            //           ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48; want src/Shop/Tea/Infuse/Cup.cs:25 (#1136)
        }
        return total;
    }

    public int Pattern(object o)
    {
        if (o is Mug cup)
        {
            return cup.Strength;
            //         ^ d: picker src/Shop/Tea/Infuse/Cup.cs:7, src/Shop/Tea/Infuse/Cup.cs:27; want src/Shop/Tea/Infuse/Cup.cs:27 (#1136)
        }
        return this.cup.Sip();
        //          ^ d: src/Shop/Tea/Infuse/Tasting.cs:21
        //              ^ d: src/Shop/Tea/Infuse/Cup.cs:48
    }

    public int Refills(Cup cup)
    {
        cup = cup.Refill();
        //        ^ d: src/Shop/Tea/Infuse/Cup.cs:20
        return cup.Warm() + cup.Add(cup);
        //         ^ d: src/Shop/Tea/Infuse/Cup.cs:41
        //                      ^ d: src/Shop/Tea/Infuse/Cup.cs:18
    }

    public int Split()
    {
        var (mug, stove) = (new Mug(), new Stove());
        return mug.Sip() + stove.Sip();
        //         ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48; want src/Shop/Tea/Infuse/Cup.cs:25 (#1136)
        //                       ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48; want src/Shop/Tea/Infuse/Cup.cs:48 (#1136)
    }

    public int Inherited(Mug mug, ISteeper steeper)
    {
        return mug.Count + mug.Add(mug) + steeper.Steep() + steeper.Strength;
        //         ^ d: picker src/Shop/Tea/Infuse/Tasting.cs:15, src/Shop/Pricing/Pricing.cs:53, src/Shop/Rules/Crates.cs:32, src/Shop/Rules/Crates.cs:76, src/Shop/Tea/Infuse/Cup.cs:12; want src/Shop/Tea/Infuse/Cup.cs:12 (#1137)
        //                     ^ d: src/Shop/Tea/Infuse/Cup.cs:18
        //                                        ^ d: src/Shop/Tea/Infuse/Cup.cs:5
        //                                                          ^ d: src/Shop/Tea/Infuse/Cup.cs:7
    }

    public int Local(Cup cup)
    {
        int Warm() => 9;
        return Warm() + cup.Warm();
        //     ^ d: src/Shop/Tea/Infuse/Tasting.cs:117
        //                  ^ d: src/Shop/Tea/Infuse/Cup.cs:41
    }

    public int Card(Glass glass)
    {
        var text = $"""
            public int Pour(int size) => {glass.Size()};
            public class Glass
            """;
        return text.Length + glass.Pour(1) + new Mug().Count;
        //                         ^ d: src/Shop/Tea/Infuse/Cup.cs:56
        //                                       ^ d: picker src/Shop/Tea/Infuse/Tasting.cs:13, src/Shop/Tea/Infuse/Cup.cs:23; want src/Shop/Tea/Infuse/Cup.cs:23 (#1137)
    }

    public int Beyond(string name, Brewer brewer)
    {
        var told = new StringBuilder().Append(name).Append("!");
        //                                          ^ d: none
        brewer.Build()
            // a note between the links
            .Sip();
        //   ^ d: src/Shop/Tea/Infuse/Cup.cs:14
        return new List<Cup>().Find(c => true).Sip() + told.Length;
        //                                     ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48
    }
}
