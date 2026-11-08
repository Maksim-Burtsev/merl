namespace Shop.Spout;

public class Carafe
{
    public virtual void Pour(int n) { }
}

public class Funnel : Carafe
{
    public void Pour(int a, int b) { }

    public void Go()
    {
        this.Pour(1);
        //   ^ d: src/Shop/Spout/Overloads.cs:5
        this.Pour(1, 2);
        //   ^ d: src/Shop/Spout/Overloads.cs:10
    }
}

public class Dipper : Carafe
{
    public void Pour(string s) { }

    public void Go()
    {
        if (true) Pour(1);
        //        ^ d: src/Shop/Spout/Overloads.cs:5
        if (true) Pour("x");
        //        ^ d: src/Shop/Spout/Overloads.cs:23
        this.Pour(1);
        //   ^ d: src/Shop/Spout/Overloads.cs:5
    }
}

public class Ladle : Carafe
{
    public void Pour(long n) { }

    public void Go()
    {
        this.Pour(1);
        //   ^ d: src/Shop/Spout/Overloads.cs:38
    }
}

public class Bowl
{
    public void Pour(string s) { }

    public void Go()
    {
        this.Pour(1);
        //   ^ d: src/Shop/Spout/Overloads.cs:49
    }
}

public class Pitcher
{
    public void Go(Dipper dipper, Funnel funnel)
    {
        dipper.Pour(1);
        //     ^ d: src/Shop/Spout/Overloads.cs:5
        dipper.Pour("x");
        //     ^ d: src/Shop/Spout/Overloads.cs:23
        funnel.Pour(1);
        //     ^ d: src/Shop/Spout/Overloads.cs:5
        new Dipper().Pour(2);
        //           ^ d: src/Shop/Spout/Overloads.cs:5
    }
}
