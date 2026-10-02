# The shop (#429): a module over the files it includes.
module Shop

export Tariff, Coupon, gross, weigh!
#                             ^ d: src/tariff.jl:57
#      ^ d: src/tariff.jl:7

const LIMIT = 10
config = Dict(:currency => "EUR")

include("money.jl")
include("tariff.jl")
#        ^ d: src/tariff.jl:1

baremodule Core2
#          ^ d: src/Shop.jl:15
end

end
