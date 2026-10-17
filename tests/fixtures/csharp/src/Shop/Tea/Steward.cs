namespace Shop.Tea;

/*
class Samovar
*/
public class Steward
{
    public int Serve(Samovar samovar) => samovar.Drain();
    //                                           ^ d: src/Shop/Tea/Urn.cs:5
}
