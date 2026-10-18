-- #1062: a test file's top-level locals, private to it.
local assert = require('luassert')
local brew_global = brew_global
return { assert = assert, brew_global = brew_global }
