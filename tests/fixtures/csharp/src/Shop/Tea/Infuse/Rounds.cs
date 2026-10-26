using System.Collections.Generic;
using System.Linq;
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
}
