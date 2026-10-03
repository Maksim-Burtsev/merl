-module(ranch).
-export([start_listener/3]).

start_listener(Ref, Transport, Opts) ->
    {ok, Ref, Transport, Opts}.
