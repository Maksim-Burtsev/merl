namespace Shop.Trestles
{
    public class Plinth : Trestle<Dowel>
    {
        public void Tidy()
        {
            Crown().Sand();
            //      ^ d: src/Shop/Trestles/Dowel.cs:5
        }
    }

    public class Cradle : Plinth
    {
    }

    public class Bracket<U> : Trestle<U>
    {
        public void Tidy()
        {
            Crown().Sand();
            //      ^ d: picker src/Shop/Trestles/Dowel.cs:5, src/Shop/Trestles/Plank.cs:5
        }
    }
}
