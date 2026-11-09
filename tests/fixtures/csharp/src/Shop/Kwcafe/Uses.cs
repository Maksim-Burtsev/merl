namespace Shop.Kwcafe;

public static class Uses
{
    public static int Serve()
    {
        var mugful = 5;
        var pot = new Kwpot();
        Brewer.Kwbrew(mugful: mugful);
        //            ^ d: src/Shop/Kwcafe/Brewer.cs:7
        //                    ^ d: src/Shop/Kwcafe/Uses.cs:7
        pot.Kwbrew(mugful: 4);
        //         ^ d: src/Shop/Kwcafe/Pot.cs:7
        var said = $"{Brewer.Kwbrew(mugful: mugful)} {{Brewer.Kwbrew(mugful: 1)}}";
        //                          ^ d: src/Shop/Kwcafe/Brewer.cs:7
        //                                  ^ d: src/Shop/Kwcafe/Uses.cs:7
        //                                                           ^ d: src/Shop/Kwcafe/Uses.cs:7
        return Brewer.Kwbrew(mugful);
        //                   ^ d: src/Shop/Kwcafe/Uses.cs:7
    }
}
