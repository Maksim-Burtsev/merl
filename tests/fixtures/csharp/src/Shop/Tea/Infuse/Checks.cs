using FluentValidation;

namespace Shop.Tea.Infuse;

public class CupValidator : AbstractValidator<Cup>
{
    public CupValidator()
    {
        RuleFor(cup => cup.Sip()).NotEmpty();
        //                 ^ d: src/Shop/Tea/Infuse/Cup.cs:14
        RuleForEach((cup) => cup.Sip());
        //                       ^ d: src/Shop/Tea/Infuse/Cup.cs:14
        When(cup => cup.Sip() > 0, () => { });
        //              ^ d: src/Shop/Tea/Infuse/Cup.cs:14
        RuleFor(cup =>
            cup.Sip());
        //      ^ d: src/Shop/Tea/Infuse/Cup.cs:14
        RuleFor(cup => cup.Refill()).Must(cup => cup.Sip() > 0);
        //                                           ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48
        Include(cup => cup.Sip());
        //                 ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48
    }
}

public class StoveValidator : FluentValidation.AbstractValidator<Stove>
{
    public StoveValidator(Platter platter)
    {
        RuleFor(stove => stove.Sip());
        //                     ^ d: src/Shop/Tea/Infuse/Cup.cs:48
        platter.RuleFor(stove => stove.Sip());
        //                             ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48
    }
}

public class MugValidator : AbstractValidator<Mug>, ISteeper
{
    public int Strength => 0;

    public MugValidator()
    {
        RuleFor(mug => mug.Sip());
        //                 ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48
    }
}

public class AnyValidator<T> : AbstractValidator<T> where T : Cup
{
    public AnyValidator()
    {
        RuleFor(any => any.Sip());
        //                 ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48
    }
}

public class PlatterCheck : Platter
{
    public PlatterCheck()
    {
        RuleFor(cup => cup.Sip());
        //                 ^ d: picker src/Shop/Tea/Infuse/Cup.cs:14, src/Shop/Tea/Infuse/Cup.cs:25, src/Shop/Tea/Infuse/Cup.cs:48
    }
}
