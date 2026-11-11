public class Feast
{
    public void Run()
    {
        var c = new Kylix();
        c.Libate();
        //^ d: src/Shop/Kylix.cs:1
        var s = new Skyphos();
        s.Libate();
        //^ d: src/Shop/Vessels.cs:1
        var r = new Rhyton();
        r.Libate();
        //^ d: src/Shop/Vessels.cs:2
        var k = new Kantharos();
        k.Libate();
        //^ d: src/Shop/Vessels.cs:5
    }
}
