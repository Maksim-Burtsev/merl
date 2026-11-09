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
#         ^ d: src/kwcafe/uses.jl:16

function kwlong(p; mugful = 1)
    #              ^ d: src/kwcafe/uses.jl:19
    t = (mugful = 1, b = 2)
    #    ^ d: none
    return t
end

function kwtuple(mugful)
    #            ^ d: src/kwcafe/uses.jl:26
    a = (mugful = 1,)
    #    ^ d: none
    b = (; mugful = 1)
    #      ^ d: none
    c = (; mugful)
    #      ^ d: src/kwcafe/uses.jl:26
    kwbrew(; mugful)
    #        ^ d: src/kwcafe/uses.jl:26
    d = (mugful = 2)
    #    ^ d: src/kwcafe/uses.jl:26
    e = (mugful = 2; 3)
    #    ^ d: src/kwcafe/uses.jl:26
    g = (x, mugful = 2) -> mugful
    #       ^ d: src/kwcafe/uses.jl:40
    h = ((mugful = 1, b = 2), 3)
    #     ^ d: none
    return (b = 1, mugful = 2)
    #              ^ d: none
end

function kwsaid(mugful)
    said = "mugful $(kwbrew(; mugful = mugful)) \$(kwbrew(; mugful = 1))"
    #       ^ d: src/kwcafe/uses.jl:48
    #                         ^ d: src/kwcafe/brewer.jl:3
    #                                  ^ d: src/kwcafe/uses.jl:48
    #                                                       ^ d: src/kwcafe/uses.jl:48
    told = """$(kwbrew(mugful = 4))"""
    #                  ^ d: src/kwcafe/brewer.jl:3
    bare = r"$(kwbrew(; mugful = 1))" * "$(mugful)"
    #                   ^ d: src/kwcafe/uses.jl:48
    #                                      ^ d: src/kwcafe/uses.jl:48
    return said * told * bare
end
