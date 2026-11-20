namespace App.Plumes;

public class List<T>
{
    public Blotter this[int i] => new Blotter();
}

public class Plume
{
    public void Scrawl() { }
}

public class Blotter
{
    public void Scrawl() { }
}

// A project `List<T>` whose indexer hands out a `Blotter`: `plumes[0]` is no `Plume` (#1610).
public class PlumeRack
{
    public void Write(List<Plume> plumes)
    {
        plumes[0].Scrawl();
        //        ^ d: !jump
    }
}
