-- #1062: a test file's top-level locals, private to it.
local assert = require('luassert')
local brew_global = brew_global
local rinse = 3
return { assert = assert, brew_global = brew_global, rinse = rinse }
