module Shapes  # f: 1-93

export area, Circle  # f:

abstract type Shape end  # f:

struct Circle <: Shape  # f: 7-9
    r::Float64  # f:
end  # f:

mutable struct Box{T}  # f: 11-14
    w::T  # f:
    h::T  # f:
end  # f:

#=  # f:
function not_code()  # f:
end  # f:
#= nested =#  # f:
=#  # f:

const DOC = """  # f:
if this were code  # f:
end  # f:
"""  # f:

"""  # f:
    area(s)  # f:

The area of `s`; `end` here is prose.  # f:
"""  # f:
function area(c::Circle)  # f: 32-34
    pi * c.r^2  # f: 32-34
end  # f: 32-34

function scan(xs, n)  # f: 36-80
    last = xs[end]  # f: 36-80
    first = xs[begin]  # f: 36-80
    mid = xs[begin:end]  # f: 36-80
    lim = xs[min(end, 3)]  # f: 36-80
    sym = (:end, :begin, :function)  # f: 36-80
    s = "a $(join(["x", "end"], ", ")) b"  # f: 36-80
    c = 'e'  # f: 36-80
    t = xs'  # f: 36-80
    if n > 0  # f: 45-51
        n -= 1  # f: 36-80
    elseif n < 0  # f: 36-80
        n += 1  # f: 36-80
    else  # f: 36-80
        n = 0  # f: 36-80
    end  # f: 36-80
    for x in xs  # f: 52-54
        n += x  # f: 36-80
    end  # f: 36-80
    while n > 10  # f: 55-57
        n -= 10  # f: 36-80
    end  # f: 36-80
    evens = [x for x in xs if iseven(x)]  # f: 36-80
    total = sum(x  # f: 36-80
                for x in xs)  # f: 36-80
    try  # f: 61-67
        error("end")  # f: 36-80
    catch e  # f: 36-80
        n = 0  # f: 36-80
    finally  # f: 36-80
        n += 1  # f: 36-80
    end  # f: 36-80
    let y = n  # f: 68-70
        n = y + 1  # f: 36-80
    end  # f: 36-80
    map(xs) do x  # f: 71-73
        x + 1  # f: 36-80
    end  # f: 36-80
    tasks = [begin  # f: 74-76
                 x * 2  # f: 36-80
             end  # f: 36-80
             for x in xs]  # f: 36-80
    r = r"end$"i  # f: 36-80
    return n  # f: 36-80
end  # f: 36-80

macro twice(ex)  # f: 82-87
    quote  # f: 83-86
        $(esc(ex))  # f: 82-87
        $(esc(ex))  # f: 82-87
    end  # f: 82-87
end  # f: 82-87

v = begin  # f: 89-91
    1 + 2  # f:
end  # f:

end  # f:
