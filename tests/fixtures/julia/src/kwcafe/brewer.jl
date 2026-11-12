const mugful = 3

kwbrew(; mugful = 0) = mugful

struct Kwpot
    mugful::Int
end

kwpour(p::Kwpot; mugful = 2) = mugful
