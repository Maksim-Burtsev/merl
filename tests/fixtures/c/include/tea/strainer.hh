namespace strainer
{
    class Mesh
    {
    public:
        double finest (
            int a
        ) const
        {
            return a;
        }
    };

    double finest (
        int a,
        int b
    )
    {
        return a + b;
    }

    double coarsest (
        int a
    )
    {
        return a;
    }
}
