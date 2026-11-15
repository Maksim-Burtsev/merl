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
        double shake (int a) const { return a; }
        static double tilt (int a) { return a; }
        double sluice () const;
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
