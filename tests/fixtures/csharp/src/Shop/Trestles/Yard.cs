namespace Shop.Trestles
{
    public class Yard
    {
        public void Run(Bracket<Dowel> b)
        {
            var p = new Plinth();
            p.Crown().Sand();
            //        ^ d: src/Shop/Trestles/Dowel.cs:5
            p.Item.Sand();
            //     ^ d: src/Shop/Trestles/Dowel.cs:5
            var c = new Cradle();
            c.Crown().Sand();
            //        ^ d: src/Shop/Trestles/Dowel.cs:5
            p.Grab(new Dowel()).Sand();
            //                  ^ d: picker src/Shop/Trestles/Dowel.cs:5, src/Shop/Trestles/Plank.cs:5
        }
    }
}
