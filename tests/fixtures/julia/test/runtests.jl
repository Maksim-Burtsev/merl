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

