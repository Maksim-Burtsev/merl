using System.Collections.Generic;

namespace Shop.Jugs;

public class JugShelf
{
    private readonly Dictionary<string, Jug> byName = new();
    private readonly IReadOnlyDictionary<string, Griddle> griddles = new Dictionary<string, Griddle>();
    private readonly Dictionary<string, List<Jug>> rows = new();

    public void Heat()
    {
        byName["a"].Boil();
        //          ^ d: src/Shop/Jugs/Jug.cs:5
        griddles["a"]?.Boil();
        //             ^ d: src/Shop/Jugs/Jug.cs:19
        var one = byName["b"];
        one.Boil();
        //  ^ d: src/Shop/Jugs/Jug.cs:5
        rows["a"][0].Boil();
        //           ^ d: !jump
    }
}
