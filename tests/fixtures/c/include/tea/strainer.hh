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

    template <typename K, typename C>
    class Tamis
    {
    public:
        void sift (const K& k);
    private:
        bool muddle (const K& k) const { return k == k; }
    };

    class Scoop
    {
    public:
        Scoop ();
        explicit operator bool () const;
        void tip () __attribute__((noinline));
    private:
        int brim () const { return level; }
        void dredge () { level++; }
        int level;
    };
}
