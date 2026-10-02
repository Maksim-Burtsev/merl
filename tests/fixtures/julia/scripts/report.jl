include("../src/money.jl")
#               ^ d: src/money.jl:1
using DataFrames
#     ^ d: none
import DataFrames: select
#                  ^ d: none
import Base: show
#            ^ d: src/tariff.jl:41
export report
#      ^ d: scripts/report.jl:12

function report(xs)
#        ^ d: scripts/report.jl:12
    m = total(xs)
#   ^ d: scripts/report.jl:14
#       ^ d: src/money.jl:18
    @check m.cents > 0
#    ^ d: src/money.jl:27
#          ^ d: scripts/report.jl:14
#          status: local
#            ^ d: src/money.jl:11
    println(format_price(m))
#           ^ d: src/money.jl:15
#           status: format_price: by name, 1 match
#                        ^ d: scripts/report.jl:14
    DataFrame(cents = [m.cents])
#   ^ d: none
end

function sample(parcel, tariff)
    weigh!(parcel)
#   ^ d: src/tariff.jl:57
    weigh(parcel)
#   ^ d: src/tariff.jl:52
    t = Tariff(1.0, "base")
#   ^ d: scripts/report.jl:35
#       ^ d: src/tariff.jl:7
    rate(t) == rate(tariff)
#   ^ d: picker src/tariff.jl:30, src/tariff.jl:33
#                   ^ d: scripts/report.jl:30
    parcel.grams = 1
#          ^ d: none
#   ^ d: scripts/report.jl:30
    parcel[1] = 2
#   ^ d: scripts/report.jl:30
    describe(t; verbose = 1)
#               ^ d: none
    w = @check t.rate > 0
#   ^ d: scripts/report.jl:48
#        ^ d: src/money.jl:27
    select(t)
#   ^ d: none
    DataFrames.select(t)
#   ^ d: none
#              ^ d: none
    LIMIT + config[:currency]
#   ^ d: src/Shop.jl:8
#           ^ d: src/Shop.jl:9
end

function extra(t, xs)
    Base.show(stdout, t)
#   ^ d: none
    sq(v) = v^2
#           ^ d: scripts/report.jl:64
    xs = sort(xs)
#             ^ d: scripts/report.jl:61
    map(rate -> rate * 2, xs)
#               ^ d: scripts/report.jl:68
end
