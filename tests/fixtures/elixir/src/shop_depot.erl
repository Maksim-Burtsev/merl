-module(shop_depot).
-export([ship/1]).

ship(P) ->
    G = shop_parcel:grams(P),
    %               ^ d: picker src/shop_parcel.erl:33, src/shop_parcel.erl:37
    %   ^ d: src/shop_parcel.erl:1
    ?MODULE:local(G),
    %       ^ d: src/shop_depot.erl:18
    ranch:start_listener(depot, tcp, #{}),
    %     ^ d: deps/ranch/src/ranch.erl:4
%   ^ d: deps/ranch/src/ranch.erl:1
    stamp(),
%   ^ d: picker src/shop_parcel.erl:83
    local(G).
%   ^ d: src/shop_depot.erl:18

local(G) ->
    G.

retry(G) ->
    missing_mod:local(G).
    %           ^ d: picker src/shop_depot.erl:18, src/shop_parcel.erl:91
