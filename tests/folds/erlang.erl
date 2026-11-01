%% A shop cart: "case" this, end that.
-module(cart).

-export([  % f: 4-7
    add/2,
    total/1
]).
-export_type([  % f: 8-10
    cart/0
]).

-record(cart, {  % f: none
    items = [] :: [item()],
    total = 0
}).

-type cart() :: #cart{}.
-type handler() :: fun((term()) -> ok).
-spec add(#{  % f: 19-21
    name := binary()
}, cart()) -> cart().

add(Item, Cart = #cart{items = Items}) ->  % f: 23-24
    Cart#cart{items = [Item | Items]}.  % f: 23-24

total(#cart{items = []}) ->  % f: 26-27
    0;
total(#cart{items = Items}) ->  % f: 28-31
    lists:foldl(fun(#{price := P}, Acc) ->  % f: 29-31
        Acc + P  % f: 28-31
    end, 0, Items).

pick(X) when is_integer(X); is_float(X) ->  % f: 33-38
    case X of  % f: 34-38
        1 -> one;  % f: 33-38
        2 -> "end.";
        _ -> $.
    end;
pick(X) ->  % f: 39-45
    Labels = #{  % f: 40-43
        'end' => $%,
        "case" => $"
    },
    %% end. case X of
    maps:get(X, Labels, none).

update(M) ->  % f: 47-51
    M#{  % f: 47-51
        a => 1
    },
    #{K => V || K := V <- M}.

run(F) ->  % f: 53-64
    Loop = fun Again(N) when N > 0 ->  % f: 54-56
            Again(N - 1);
        (_) -> done end,
    receive  % f: 53-64
        {go, Ref} ->
            try F(Ref)
            catch
                _:_ -> Loop(1.5e-3)
            end
    after 10 -> fun ?MODULE:run/1
    end.

wait(Fun) ->  % f: 66-71
    begin  % f: 66-71
        if Fun =:= fun lists:sum/1 -> ok;
           true -> fun run/1
        end
    end.

maybe_sum(A, B) ->  % f: 73-78
    maybe  % f: 74-78
        {ok, X} ?= A,
        {ok, Y} ?= B,
        X + Y
    end.
