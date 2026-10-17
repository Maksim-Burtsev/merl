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
  --          ^ d: lua/tea/teapot.lua:8
end

return Teapot
