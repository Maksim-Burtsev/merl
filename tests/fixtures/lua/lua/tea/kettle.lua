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
  --     ^ d: lua/tea/kettle.lua:14
end

M.Kettle = Kettle
M.steep = pour
return M
