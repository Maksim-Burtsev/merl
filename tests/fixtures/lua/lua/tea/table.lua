-- #931: what Lua's own scoping makes hard for `d`.
local kettle = require("tea.kettle")
local Teapot = require("tea.teapot")

local T = {}

local code = [==[
function Teapot:pour(cups)
local function fib(x)
]==]

--[[
local function pour(cups)
function T.redeclare()
]]

function T.serve(kettle)
  return kettle:pour(1)
  --     ^ d: lua/tea/table.lua:17
  --            ^ d: !jump
end

function T.brew()
  return kettle.pour(1) + kettle.steep(2)
  --            ^ d: lua/tea/kettle.lua:18
  --                             ^ d: lua/tea/kettle.lua:24
end

function T.pots()
  local pot = Teapot.new()
  --          ^ d: lua/tea/table.lua:3
  --                 ^ d: lua/tea/teapot.lua:4
  local k = kettle.Kettle.new(1)
  --               ^ d: lua/tea/kettle.lua:3
  --                      ^ d: lua/tea/kettle.lua:6
  return pot:pour(1) + k:pour(1) + pot:decant()
  --         ^ d: lua/tea/teapot.lua:8
  --                     ^ d: lua/tea/kettle.lua:10
  --                                   ^ d: lua/tea/teapot.lua:12
end

function T.redeclare()
  local n = 1
  local n = n + 1
  --        ^ d: lua/tea/table.lua:43
  return n
  --     ^ d: lua/tea/table.lua:44
end

local fact = function(x)
  return x > 1 and fact(x - 1) or 1
  --               ^ d: none
end

local function fib(x)
  return x > 1 and fib(x - 1) or 1
  --               ^ d: lua/tea/table.lua:55
end

local later
function later()
  return code
end

function T.loop(items)
  local i = 0
  for i = 1, #items do
    print(i)
    --    ^ d: lua/tea/table.lua:67
  end
  return i
  --     ^ d: lua/tea/table.lua:66
end

function brew_global()
  return later() + fact(3) + fib(3)
  --     ^ d: lua/tea/table.lua:60
  --               ^ d: lua/tea/table.lua:50
  --                         ^ d: lua/tea/table.lua:55
end

function T.call()
  return brew_global() + T.redeclare()
  --     ^ d: lua/tea/table.lua:75
  --                       ^ d: lua/tea/table.lua:42
end

return T
