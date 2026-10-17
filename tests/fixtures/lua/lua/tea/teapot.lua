local Teapot = {}
Teapot.__index = Teapot

function Teapot.new()
  return setmetatable({}, Teapot)
end

function Teapot:pour(cups)
  return cups * 2
end

function Teapot.decant(self)
  return self:pour(1)
  --          ^ d: picker lua/tea/kettle.lua:10, lua/tea/kettle.lua:14, lua/tea/kettle.lua:18, lua/tea/teapot.lua:8; want lua/tea/teapot.lua:8 (#952)
end

return Teapot
