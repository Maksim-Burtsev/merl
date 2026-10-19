-- #1062: another file's top-level `local` is no definition of a bare name here.
-- #1112: nor is another file's table key.
local C = {}

function C.fill(n)
  assert(type(n) == 'number')
  -- ^ d: none
  return brew_global() + n
  --     ^ d: lua/tea/table.lua:75
end

function C.pour()
  return pour_global() + steep_global()
  --     ^ d: lua/tea/spec.lua:10
  --                     ^ d: lua/tea/spec.lua:12
end

return C
