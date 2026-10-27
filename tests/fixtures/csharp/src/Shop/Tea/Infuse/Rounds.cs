using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;

namespace Shop.Tea.Infuse;

public class Platter
{
    public Task<IReadOnlyCollection<Cup>> MugsAsync() => null!;

    public IEnumerable<Cup> Mugs() => [];

    public List<Cup> Cups { get; } = new();

    public Dictionary<string, Cup> Priced { get; } = new();

    public IEnumerable<Cup> Pick(int n) => [];

    public IEnumerable<Stove> Pick(string s) => [];

    public IEnumerable<Cup> Stoked() => [];
}

public static class PlatterExtensions
{
    public static Stove Last(this IEnumerable<Cup> mugs) => new Stove();

    public static Stove Heat(this Cup cup) => new Stove();
}

public class Saucers : List<Cup>
{
}

public static class SaucersExtensions
{
    public static bool TrueForAll(this Saucers saucers, Predicate<Cup> match) => false;
}

public class Rounds(Platter platter)
{
    public async Task<int> Pour()
    {
        var mugs = await platter.MugsAsync();
        int total = 0;
        foreach (var mug in mugs)
        {
            total += mug.Sip();
            //           ^ d: src/Shop/Tea/Infuse/Cup.cs:14
        }
        total += platter.Cups
            .Where(c => c.Count > 0)
            //            ^ d: src/Shop/Tea/Infuse/Cup.cs:12
            .Count();
        total += platter.Mugs().FirstOrDefault()?.Sip() ?? 0;
        //                                        ^ d: src/Shop/Tea/Infuse/Cup.cs:14
        return total;
    }

    public int Unsure()
    {
        int total = platter.Priced.Count(p => p.Value.Sip() > 0);
        //                                            ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48
        total += platter.Cups.Aggregate(0, (n, c) => n + c.Sip());
        //                                                 ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48
        total += platter.Pick(1).First().Sip();
        //                               ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48
        return total + platter.Stoked().Last().Sip();
        //                                     ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48
    }

    public async Task<int> Served(string json)
    {
        int total = 0;
        var stoves = platter.Cups.Select(c => c.Heat());
        foreach (var stove in stoves)
        {
            total += stove.Warm();
            //             ^ d: src/Shop/Tea/Infuse/Cup.cs:46
        }
        total += platter.Cups.First().Heat().Warm();
        //                                   ^ d: src/Shop/Tea/Infuse/Cup.cs:46
        var made = (await platter.MugsAsync()).Select(m => new Stove()).ToList();
        for (var i = 0; i < made.Count; i++)
        {
            var one = made[i];
            total += one.Sip();
            //           ^ d: src/Shop/Tea/Infuse/Cup.cs:48
        }
        var poured = JsonSerializer.Deserialize<Cup[]>(json) ?? Array.Empty<Cup>();
        total += poured.Count(p =>
            p.Sip() > 0);
        //    ^ d: src/Shop/Tea/Infuse/Cup.cs:14
        return total;
    }

    public int Doubtful(string json)
    {
        var mixed = JsonSerializer.Deserialize<Cup[]>(json) ?? Array.Empty<Mug>();
        int total = mixed.Count(m =>
            m.Sip() > 0);
        //    ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48
        var blocks = platter.Cups.Select(c => { return c.Heat(); });
        foreach (var block in blocks)
        {
            total += block.Warm();
            //             ^ d: picker src/Shop/Tea/Infuse/Cup.cs:41, src/Shop/Tea/Infuse/Cup.cs:46
        }
        return total;
    }

    public bool Stacked(Saucers saucers)
    {
        return saucers.TrueForAll(c => c.Count > 0);
        //             ^ d: none
    }
}
