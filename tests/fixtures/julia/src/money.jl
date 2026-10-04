"""
    Money(cents)

An amount in cents; a docstring that reads like code:

    struct Money end
    function total(xs) end
"""
struct Money
#      ^ d: src/money.jl:9
    cents::Int
#   ^ d: src/money.jl:11
end

format_price(m::Money) = string("\$", m.cents / 100)
#                                     ^ d: src/money.jl:15

function total(xs::Vector{Money})
#              ^ d: src/money.jl:18
    Money(sum(m.cents for m in xs))
#   ^ d: src/money.jl:9
#             ^ d: src/money.jl:20
#                         ^ d: src/money.jl:20
#                              ^ d: src/money.jl:18
end

macro check(ex)
    :($(esc(ex)) || error("check failed"))
#           ^ d: src/money.jl:27
end
