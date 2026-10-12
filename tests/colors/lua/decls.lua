#!/usr/bin/env lua
--- A module of shapes, the declarations a Lua file is made of.
-- @module shapes

local M = {}
local VERSION = "1.0"
local unpack = table.unpack or unpack
local count <const> = 0
local file <close> = nil

---@class Shape
---@field kind string
---@field size number
local Shape = {}
Shape.__index = Shape

---@param kind string the kind of shape
---@param size? number
---@return Shape
function Shape.new(kind, size)
  local self = setmetatable({}, Shape)
  self.kind = kind
  self.size = size or 1
  return self
end

function Shape:area()
  if self.kind == "square" then
    return self.size ^ 2
  elseif self.kind == "circle" then
    return math.pi * self.size * self.size
  else
    error("unknown kind: " .. tostring(self.kind))
  end
end

M.describe = function(shape, ...)
  local extra = select("#", ...)
  return string.format("%s of %d (%5.2f)", shape.kind, extra, shape:area())
end

local defaults = {
  name = "box",
  sizes = { 1, 2.5, 0x10, 1e3 },
  ["with space"] = true,
  render = function(s) return s end,
  [1] = nil,
}

M.Shape, M.defaults, M._VERSION = Shape, defaults, _VERSION
return M
