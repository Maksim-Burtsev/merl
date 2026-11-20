const mugful = 3

kwbrew(; mugful = 0) = mugful

struct Kwpot
    mugful::Int
end

kwpour(p::Kwpot; mugful = 2) = mugful

kwtwin(p::Kwpot; mugful = 1) = mugful
kwtwin(p::Int; mugful = 2) = mugful

function kwfill(p::Kwpot; cups = 1,
                mugful = 2)
    return cups + mugful
end
