using System.Collections.Generic;

namespace Shop.Jugs;

public class JugRow
{
    private readonly List<Jug> jugs = new();

    public void Heat(List<Jug> kettles, IReadOnlyList<Griddle> griddles, List<List<Jug>> grid)
    {
        kettles[0].Boil();
        //         ^ d: src/Shop/Jugs/Jug.cs:5
        jugs[0].Boil();
        //      ^ d: src/Shop/Jugs/Jug.cs:5
        griddles[0]?.Boil();
        //           ^ d: src/Shop/Jugs/Jug.cs:19
        var local = new List<Jug>();
        local[0].Boil();
        //       ^ d: src/Shop/Jugs/Jug.cs:5
        var one = kettles[1];
        one.Boil();
        //  ^ d: src/Shop/Jugs/Jug.cs:5
        grid[0][1].Boil();
        //         ^ d: !jump
    }
}

public class JugTray<Jug>
{
    private readonly List<Jug> items = new();

    public void Heat()
    {
        items[0].Boil();
        //       ^ d: !jump
    }
}
