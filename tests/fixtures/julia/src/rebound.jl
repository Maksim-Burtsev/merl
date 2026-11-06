# Rebound names: a `let` binding, on the cursor's line before the cursor or on an enclosing
# `let` line, and an assignment ended by `;` earlier on the line are not the outer binding;
# the control rows keep master's answer on a plain read, a closed block, a `let`'s right side,
# a keyword argument, a comparison, a comment and a string.
struct Flask
    sip::Int
end
struct Jug
    sip::Int
end
swig() = 1

function plain(flask::Flask)
    flask.sip
    #     ^ d: src/rebound.jl:6
    flask
#   ^ d: src/rebound.jl:13
end

function let_lines(flask::Flask)
    let flask = Jug(1)
        flask.sip
        #     ^ d: picker src/rebound.jl:6, src/rebound.jl:9
#       ^ d: src/rebound.jl:21
    end
    let y = 1, swig = 2
        swig
#       ^ d: src/rebound.jl:26
    end
    let y = swig
        swig
#       ^ d: src/rebound.jl:11
    end
end

function let_one(flask::Flask)
    let flask = Jug(1); flask end
    #                   ^ d: src/rebound.jl:37
    let flask = Jug(1); flask end; flask
    #                              ^ d: src/rebound.jl:36
    let flask = flask.sip; flask end
    #           ^ d: src/rebound.jl:36
    flask.sip + let flask = Jug(1); flask end
    #                               ^ d: src/rebound.jl:43
#   ^ d: src/rebound.jl:36
    let Flask = 3; Flask end
    #              ^ d: src/rebound.jl:46
end

function assigned(flask::Flask, xs)
    swig = 2; swig
    #         ^ d: src/rebound.jl:51
    flask = Jug(1); flask
    #               ^ d: src/rebound.jl:53
end

function controls(flask::Flask, xs)
    # let flask = Jug(1); flask
    f(xs; flask = 1); flask
    #                 ^ d: src/rebound.jl:57
    flask == 1; flask
    #           ^ d: src/rebound.jl:57
    for x in xs; flask = 1; end; flask
    #                            ^ d: src/rebound.jl:57
    s = "flask = 1; "; flask
    #                  ^ d: src/rebound.jl:57
end
