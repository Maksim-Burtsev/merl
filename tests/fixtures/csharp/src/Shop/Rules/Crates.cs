namespace Shop.Rules;

// A positional record's header declares its properties, on one line or over several.
public record Crate(int Slots, Spot Corner);

public record class Pallet(
    [property: Obsolete("stack")]
    int Layers,
    Crate Bin) : Crate(1, new Spot(0, 0));

// An ordinary method's and a class primary constructor's parameters are no members.
public class Barge(int Keel)
{
    public int Load(int Cargo, int Tare) => Cargo;

    public int Moor(
        int Ballast,
        Crate Hold) => Ballast;
}

public class Hull(
    int Rivets,
    Crate Plates) : Barge(Rivets)
{
    public int Peer(dynamic other) => other.Rivets + other.Plates;
    //                                      ^ d: none
    //                                                     ^ d: none
}

public class Docker
{
    public int Count(dynamic box)
    {
        var a = box.Slots;
        //          ^ d: src/Shop/Rules/Crates.cs:4
        var b = box.Corner;
        //          ^ d: src/Shop/Rules/Crates.cs:4
        var c = box.Layers;
        //          ^ d: src/Shop/Rules/Crates.cs:8
        var d = box.Bin;
        //          ^ d: src/Shop/Rules/Crates.cs:9
        var e = box.Keel;
        //          ^ d: none
        var f = box.Tare;
        //          ^ d: none
        var g = box.Ballast;
        //          ^ d: none
        return box.Hold;
        //         ^ d: none
    }

    public int Typed(Pallet pallet, Barge barge)
    {
        var a = pallet.Layers;
        //             ^ d: src/Shop/Rules/Crates.cs:8
        return barge.Ballast;
        //           ^ d: none
    }
}
