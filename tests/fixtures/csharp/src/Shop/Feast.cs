public class Feast
{
    public void Run()
    {
        var c = new Kylix();
        c.Libate();
        //^ d: picker src/Shop/Kylix.cs:1, src/Shop/Vessels.cs:1, src/Shop/Vessels.cs:2, src/Shop/Vessels.cs:5; want src/Shop/Kylix.cs:1 (#1486)
        var s = new Skyphos();
        s.Libate();
        //^ d: picker src/Shop/Kylix.cs:1, src/Shop/Vessels.cs:1, src/Shop/Vessels.cs:2, src/Shop/Vessels.cs:5; want src/Shop/Vessels.cs:1 (#1486)
        var r = new Rhyton();
        r.Libate();
        //^ d: picker src/Shop/Kylix.cs:1, src/Shop/Vessels.cs:1, src/Shop/Vessels.cs:2, src/Shop/Vessels.cs:5; want src/Shop/Vessels.cs:2 (#1486)
        var k = new Kantharos();
        k.Libate();
        //^ d: src/Shop/Vessels.cs:5
    }
}
