function kwserve()
    mugful = 5
    pot = Kwpot(1)
    kwbrew(; mugful = mugful)
    #        ^ d: src/kwcafe/brewer.jl:3
    #                 ^ d: src/kwcafe/uses.jl:2
    kwbrew(mugful = 4)
    #      ^ d: src/kwcafe/brewer.jl:3
    kwpour(pot; mugful = 4)
    #           ^ d: none
    kwbrew(mugful=mugful)
    #      ^ d: src/kwcafe/brewer.jl:3
    return pot
end

kwshort(; mugful = 0) = mugful
#         ^ d: none

function kwlong(p; mugful = 1)
    #              ^ d: src/kwcafe/uses.jl:19
    t = (mugful = 1, b = 2)
    #    ^ d: src/kwcafe/uses.jl:19; want none (#1446)
    return t
end
