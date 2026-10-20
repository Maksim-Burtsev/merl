-- #1062: a bare name whose namesakes are a table's `function S.rinse` and another file's
-- top-level local stays a picker of both.
local spare = rinse()
--            ^ d: picker lua/tea/saucer.lua:4, lua/tea/spec.lua:13
return spare
