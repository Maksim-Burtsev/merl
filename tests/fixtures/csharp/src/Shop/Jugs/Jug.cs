namespace Shop.Jugs;

public class Jug
{
    public void Boil() { }

    public static JugBuilder Builder() => new JugBuilder();
}

public class JugBuilder
{
    public JugBuilder Size(int n) => this;

    public Jug Build() => new Jug();
}

public class Griddle
{
    public void Boil() { }

    public Griddle Size(int n) => this;

    public Griddle Build() => this;
}
