-- #1290: LuaLS `---@class` / `---@field` types, and a module function's `{ __index = _M }`.
local caddy = require("tea.caddy")
local brand = require("tea.brand")

local M = {}

caddy.seal("tin")
--    ^ d: lua/tea/caddy.lua:2

---@param leaf? Leaf
---@param leaves Leaf[]
---@param mixed Leaf|nil
function M.weigh(leaf, leaves, mixed)
  local a = leaf.origin
  --             ^ d: lua/tea/_meta/leaf.lua:7
  local b = leaves.origin
  --               ^ d: none
  local c = mixed.grade
  --              ^ d: none
  return a, b, c
end

---@param leaf Leaf
function M.shadowed(leaf)
  local leaf = {}
  return leaf.origin
  --          ^ d: none
end

---@param leaves Leaf
function M.prefix(leaf)
  return leaf.grade
  --          ^ d: none
end

M.later = brand.define(function(leaf)
  return leaf.grade
  --          ^ d: none
end)

---@return Leaf?
local function pick()
  return nil
end

---@return Leaf
---@return string err
local function pick_two()
  return nil, ""
end

function M.picked()
  local l = pick()
  local m = pick_two()
  return l.grade, m.grade
  --       ^ d: lua/tea/_meta/leaf.lua:8
  --                ^ d: none
end

local Brand = brand.define("oolong")
Brand:enable()
--    ^ d: lua/tea/brand.lua:13
local Paired = brand.paired("sencha")
Paired:enable()
--     ^ d: picker lua/tea/brand.lua:13, lua/tea/brand.lua:19
local Others = brand.other()
Others:enable()
--     ^ d: picker lua/tea/brand.lua:13, lua/tea/brand.lua:19

local typedefs = require("tea.typedefs")

M.fields = {
  { id = typedefs.uuid, },
  --                ^ d: lua/tea/typedefs.lua:3
  { name = typedefs.name },
  --                ^ d: lua/tea/typedefs.lua:7
  { count = typedefs.count },
  --                 ^ d: none
}

return M
