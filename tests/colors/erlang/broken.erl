%% Broken on purpose: an open clause and string.
-module(broken).
f(X ->
    io:format("unterminated),
g() -> [1, 2
