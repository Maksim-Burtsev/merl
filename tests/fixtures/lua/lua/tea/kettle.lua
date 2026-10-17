local M = {}

local Kettle = {}
Kettle.__index = Kettle

function Kettle.new(size)
  return setmetatable({ size = size }, Kettle)
end

function Kettle:pour(cups)
  return cups * self.size
end

local function pour(cups)
  return cups
end

function M.pour(cups)
  return pour(cups) + 1
  --     ^ d: picker lua/tea/kettle.lua:10, lua/tea/kettle.lua:14, lua/tea/kettle.lua:18, lua/tea/teapot.lua:8; want lua/tea/kettle.lua:14 (#953)
end

M.Kettle = Kettle
M.steep = pour
return M
