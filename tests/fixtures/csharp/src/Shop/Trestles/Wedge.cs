namespace Shop.Trestles
{
    public class Wedge : Trestle<Plank, Dowel>
    {
        public void Tidy()
        {
            Crown().Sand();
            //      ^ d: picker src/Shop/Trestles/Dowel.cs:5, src/Shop/Trestles/Plank.cs:5
        }
    }
}
