using Microsoft.AspNetCore.Authorization;

namespace Shop.Gates
{
    using Shop.Crest;

    public class Post
    {
        [ProjectAttr(Name = "post")]
       //^ d: src/Shop/Crest/Badges.cs:3
       //            ^ d: src/Shop/Crest/Badges.cs:7
        public void Open() { }

        [Obsolete]
       //^ d: none
        public void Close() { }

        public object Clasp() => new Hasp();
        //                           ^ d: none
    }
}

namespace Shop.Gates.Back
{
    using Shop.Larder;

    public class Stile
    {
        [ProjectAttr("back")]
       //^ d: none
        public void Climb() { }

        [Authorize]
       //^ d: none
        public void Shutter() { }

        public object Clasp() => new Hasp();
        //                           ^ d: src/Shop/Gates/Kit.cs:10
    }
}
