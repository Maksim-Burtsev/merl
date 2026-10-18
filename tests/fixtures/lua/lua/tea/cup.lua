-- #1062: another file's top-level `local` is no definition of a bare name here.
local C = {}

function C.fill(n)
  assert(type(n) == 'number')
  -- ^ d: none
  return brew_global() + n
  --     ^ d: lua/tea/table.lua:75
end

return C
