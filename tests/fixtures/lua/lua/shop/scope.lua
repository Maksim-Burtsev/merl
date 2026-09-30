-- #461: a method called with `:` is no local, `until` reads its `repeat` body, a local of a
-- block the cursor has left is not its, nor is a top-level local.
local Scope = {}
local RATE = 2

function Scope:weight() return 1 end

function Scope.ship(self)
  local weight = 1
  return self:weight() + weight
  --          ^ d: picker lua/shop/scope.lua:6
end

function Scope.poll(step)
  repeat
    local done = step()
  until done
  --    ^ d: lua/shop/scope.lua:16
end

function Scope.cap(c)
  if c then
    local limit = 1
    print(limit)
  end
  return limit
  --     ^ d: none
end

function Scope.charge()
  return RATE
  --     ^ d: lua/shop/scope.lua:4
  --   status: by name
end

return Scope
