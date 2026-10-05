abstract type Pricing end
#             ^ d: src/tariff.jl:1

primitive type Grams 32 end
#              ^ d: src/tariff.jl:4

struct Tariff <: Pricing
#                ^ d: src/tariff.jl:1
    rate::Float64
#   ^ d: picker src/tariff.jl:16, src/tariff.jl:30, src/tariff.jl:33
    name
#   ^ d: src/tariff.jl:11
end

mutable struct Coupon <: Pricing
    rate::Float64
    used::Bool
end

Base.@kwdef struct Options
#                  ^ d: src/tariff.jl:20
    express::Bool = false
#   ^ d: src/tariff.jl:22
end

@enum Courier north south
#     ^ d: src/tariff.jl:26
#                   ^ d: src/tariff.jl:26

rate(t::Tariff) = t.rate
#                 ^ d: src/tariff.jl:30

function rate(c::Coupon)
    c.used ? 0.0 : c.rate
#                  ^ d: src/tariff.jl:33
end

describe(t::Tariff)::String = "tariff $(t.name)"
describe(c::Coupon) where {T} = "coupon"

function Base.show(io::IO, t::Tariff)
#             ^ d: src/tariff.jl:41
    print(io, describe(t))
#             ^ d: picker src/tariff.jl:38, src/tariff.jl:39
#                      ^ d: src/tariff.jl:41
end

gross(p::Pricing, net) = net * (1 + rate(p))
#                                   ^ d: picker src/tariff.jl:30, src/tariff.jl:33
#                        ^ d: src/tariff.jl:48

function weigh(parcel)
    parcel.grams
#          ^ d: none
end

function weigh!(parcel)
    parcel.grams = 0
#          ^ d: none
end

function (t::Tariff)(net)
#                    ^ d: src/tariff.jl:62
    gross(t, net)
#   ^ d: src/tariff.jl:48
#            ^ d: src/tariff.jl:62
#         ^ d: src/tariff.jl:62
end

#=
function hidden() end
#= nested =#
struct Hidden end
=#
