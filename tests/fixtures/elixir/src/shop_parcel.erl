-module(shop_parcel).
-export([make/1, grams/1, heavy/2, pack/2]).
-export_type([parcel/0, label/0]).
-behaviour(gen_server).

-include("shop_parcel.hrl").
%         ^ d: include/shop_parcel.hrl:1
-include_lib("ranch/include/ranch.hrl").
%                   ^ d: deps/ranch/include/ranch.hrl:1

-define(DEFAULT_GRAMS, 0).
-define(LOG(Fmt, Args), io:format(Fmt, Args)).

-type parcel() :: #parcel{}.
%                  ^ d: include/shop_parcel.hrl:1
-opaque label() :: binary().

-spec grams(parcel()) -> integer().
%     ^ d: picker src/shop_parcel.erl:33, src/shop_parcel.erl:37
%           ^ d: src/shop_parcel.erl:14

-callback audit(term()) -> integer().
%         ^ d: none

make(Ref) ->
    #parcel{ref = Ref, grams = ?DEFAULT_GRAMS}.
    %^ d: include/shop_parcel.hrl:1
    %       ^ d: include/shop_parcel.hrl:1
    %             ^ d: none
    %                  ^ d: include/shop_parcel.hrl:1
    %                           ^ d: src/shop_parcel.erl:11

grams(#parcel{grams = Grams}) ->
%             ^ d: include/shop_parcel.hrl:1
    Grams.
%   ^ d: none
grams(_) ->
    0.

heavy(P, Limit) when is_record(P, parcel) ->
    P#parcel.grams > Limit;
    %        ^ d: include/shop_parcel.hrl:1
    % ^ d: include/shop_parcel.hrl:1
heavy(_, _) ->
    false.

pack(Crate, P) ->
    ?LOG("~p~n", [?MODULE]),
    %^ d: src/shop_parcel.erl:12
    %              ^ d: none
    Size = length(Crate#crate.parcels),
    %                         ^ d: include/shop_parcel.hrl:5
    Size < ?CRATE_LIMIT andalso heavy(P, 0),
    %       ^ d: include/shop_parcel.hrl:8
    %                           ^ d: picker src/shop_parcel.erl:40, src/shop_parcel.erl:44
    Crate#crate{sealed = true, parcels = [P]}.
    %           ^ d: include/shop_parcel.hrl:6
    %                          ^ d: include/shop_parcel.hrl:5
    %     ^ d: include/shop_parcel.hrl:3

describe_parcel(Ref,
                Grams) when Grams > 0 ->
    {Ref, Grams};
describe_parcel(Ref, _) ->
    Ref.

weigh_all(Parcels) ->
    lists:map(fun grams/1, Parcels),
    %             ^ d: picker src/shop_parcel.erl:33, src/shop_parcel.erl:37
    [describe_parcel(P, 1) || P <- Parcels].
    %^ d: picker src/shop_parcel.erl:61, src/shop_parcel.erl:64

% A comment holding a """ and a $" hides nothing:
banner() -> $".
spaced() -> "a % b", #parcel{}.
seal(C) -> C#crate{sealed = true}.
%                  ^ d: include/shop_parcel.hrl:6

-define(QUOTE, """
    make(Ref) ->
    """).

stamp() -> "
make(Ref) ->
".

speed(fast) ->
    fast.
%   ^ d: none

local(X) ->
    X.

-record(tray, {top :: parcel()}).
%                     ^ d: src/shop_parcel.erl:14

fetch(P) ->
    {ok, G} = read(P),
    reply(ok).
    %     ^ d: none
