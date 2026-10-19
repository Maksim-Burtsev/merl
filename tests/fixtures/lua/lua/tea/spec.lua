-- #1062: a test file's top-level locals, private to it.
local assert = require('luassert')
local brew_global = brew_global
-- #1112: a table's key is no global; a global assigned in a block or at the top level is one.
local sandbox = setmetatable({
  -- a homebrewed assert, as Kong's sandbox has a `require = function` :)
  assert = function(v) return v end,
}, {})
if not pour_global then
  pour_global = function() return 1 end
end
steep_global = function() return 2 end
local rinse = 3
return { assert = assert, brew_global = brew_global, sandbox = sandbox, rinse = rinse }
