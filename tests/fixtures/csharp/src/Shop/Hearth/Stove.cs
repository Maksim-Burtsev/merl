namespace Shop.Hearth
{
    using Wares;

    public class Stove
    {
        public object Light()
        {
            var b = new Brazier();
            //          ^ d: src/Shop/Hearth/Brazier.cs:3
            return new Tongs();
            //         ^ d: src/Shop/Hearth/Wares.cs:3
        }
    }
}
