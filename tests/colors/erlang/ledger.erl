%% @doc A small ledger: entries, totals and a report.
-module(ledger).
-behaviour(gen_server).

-include("ledger.hrl").
-include_lib("kernel/include/logger.hrl").

-export([start_link/0, add/2, total/1, report/1]).
-export([init/1, handle_call/3, handle_cast/2]).
-export_type([entry/0]).

-define(MAX_ENTRIES, 1024).
-define(TIMEOUT(Ms), Ms * 2).

-record(entry, {name = <<>> :: binary(), amount = 0 :: integer(), tags = [] :: [atom()]}).

-type entry() :: #entry{}.
-opaque handle() :: pid().

-spec start_link() -> {ok, pid()} | {error, term()}.
start_link() ->
    gen_server:start_link({local, ?MODULE}, ?MODULE, [], []).

%% Adds one entry.
-spec add(binary(), integer()) -> ok.
add(Name, Amount) when is_binary(Name), Amount =/= 0 ->
    gen_server:cast(?MODULE, {add, #entry{name = Name, amount = Amount}});
add(_Name, _Amount) ->
    erlang:error(badarg).

total(Entries) ->
    lists:foldl(fun(#entry{amount = A}, Sum) -> Sum + A end, 0, Entries).

report(Entries) ->
    Ratio = 1.5e3,
    Mask = 16#ff,
    Letter = $z,
    Newline = $\n,
    Text = "ledger \"main\"~n",
    Bin = <<"bytes", 1:8, Rest/binary>> = <<"bytes", 1, 2>>,
    Credits = [E || E = #entry{amount = A} <- Entries, A > 0],
    Map = #{name => <<"ledger">>, count => length(Entries)},
    #{name := Name} = Map,
    Updated = Map#{count := 0},
    case length(Entries) of
        N when N > ?MAX_ENTRIES -> throw({too_many, N});
        0 -> io:format("empty~n");
        _ -> ok
    end,
    if
        Ratio >= 1.0, Mask =< 255 -> true;
        true -> false
    end,
    Fun = fun ?MODULE:total/1,
    try Fun(Credits) of
        Sum -> {ok, Sum, Letter, Newline, Text, Bin, Rest, Name, Updated}
    catch
        error:Reason:Stack -> {error, Reason, Stack}
    after
        ok
    end.

init([]) ->
    {ok, #{entries => []}}.

handle_call(total, _From, #{entries := Es} = State) ->
    {reply, total(Es), State}.

handle_cast({add, E}, #{entries := Es} = State) ->
    receive
        stop -> {stop, normal, State}
    after ?TIMEOUT(10) ->
        {noreply, State#{entries := [E | Es]}}
    end.
