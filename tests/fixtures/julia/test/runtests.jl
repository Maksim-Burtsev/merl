# Locals of a two-line method signature and of test blocks, beside namesakes in basket.jl.
Base.push!(xs::Vector, row::Tariff;
           strict::Bool = true) =
    append!(xs, row)
#               ^ d: test/runtests.jl:2

@testset "rates" begin
    t = north
    for c in (north, south)
        res = rate(c)
        res = res * 2
        @test res > 0
#             ^ d: test/runtests.jl:10
        t = c
        @test t != c
#             ^ d: test/runtests.jl:8
    end
    @testset "inner" begin
        t = south
        t = north
        @test t == north
#             ^ d: test/runtests.jl:19
    end
end

using Test: @test
test(x) = x
Union{}(a...) = throw(ArgumentError("no value of Union{}"))
rows = Union{Int, Nothing}[1, nothing]
#      ^ d: none
fun(v) = sum(v)
try
    rate(north)
catch err
    if err isa MethodError
        rethrow(err)
#               ^ d: test/runtests.jl:34
    end
end
seen = err
#      ^ d: none
