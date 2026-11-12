namespace Shop.Vessels;

public class Pour
{
    IVessel v;
    object o;

    void Casts()
    {
        ((Beaker)v).Fill();
        //          ^ d: src/Shop/Vessels/Beaker.cs:5
        ((Beaker)o).Brim();
        //          ^ d: src/Shop/Vessels/Beaker.cs:7
        (v as Flask).Fill();
        //           ^ d: src/Shop/Vessels/Flask.cs:5
        (v as Flask)?.Brim();
        //            ^ d: src/Shop/Vessels/Flask.cs:7
        v.Fill();
        //^ d: src/Shop/Vessels/IVessel.cs:5
    }

    void Early()
    {
        if (v is not Beaker g) return;
        g.Brim();
        //^ d: picker src/Shop/Vessels/Beaker.cs:7, src/Shop/Vessels/Flask.cs:7, src/Shop/Vessels/IVessel.cs:7; want src/Shop/Vessels/Beaker.cs:7 (#1494)
    }
}
