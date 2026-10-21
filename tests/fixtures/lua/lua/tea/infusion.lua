-- #1126: the third corner, its answers read from the code.
local gaiwan = require("tea.gaiwan")
local G = require("tea.gaiwan")
local whisk = require("tea.gaiwan").whisk
local ok, cjson = pcall(require, "cjson")
local Gaiwan = gaiwan.Gaiwan

local I = {}

function I.name(s)
  return string.format("%s", s) .. gaiwan.format(s)
  --            ^ d: none
  --                                      ^ d: lua/tea/gaiwan.lua:35
end

function I.collect(list, v)
  table.insert(list, v)
  --    ^ d: none
  G.insert(list, v)
  --^ d: lua/tea/gaiwan.lua:39
  return ("cup"):upper() .. tostring(v):upper()
  --             ^ d: none
  --                                    ^ d: none
end

function I.whisked(n)
  return whisk(n) + (ok and cjson.encode(n) or 0)
  --     ^ d: lua/tea/infusion.lua:4
  --                        ^ d: lua/tea/infusion.lua:5
  --                              ^ d: none
end

function I.chain()
  local cup = Gaiwan.new(3):soak(10)
  --                        ^ d: lua/tea/gaiwan.lua:11
  local poured = Gaiwan.new(2)
    :soak(5)
   --^ d: lua/tea/gaiwan.lua:11
    :unfold()
   --^ d: picker lua/tea/gaiwan.lua:16, lua/tea/gaiwan.lua:31; want lua/tea/gaiwan.lua:16 (#1208)
  return cup:unfold() + poured
  --         ^ d: picker lua/tea/gaiwan.lua:16, lua/tea/gaiwan.lua:31; want lua/tea/gaiwan.lua:16 (#1208)
end

function I.shadow()
  local gaiwan = gaiwan.Gaiwan.new(1)
  --             ^ d: lua/tea/infusion.lua:2
  --                    ^ d: lua/tea/gaiwan.lua:4
  return gaiwan:unfold()
  --            ^ d: lua/tea/gaiwan.lua:16
end

function I.swap(a, b)
  local a, b = b, a
  --           ^ d: lua/tea/infusion.lua:53
  --              ^ d: lua/tea/infusion.lua:53
  return a - b
  --     ^ d: lua/tea/infusion.lua:54
end

function I.each(cups)
  for _, cups in ipairs(cups) do
    --                  ^ d: lua/tea/infusion.lua:61
    print(cups)
    --    ^ d: lua/tea/infusion.lua:62
  end
end

function I.inherit()
  local s = gaiwan.Sencha.new(1)
  s:soak(2)
  --^ d: lua/tea/gaiwan.lua:11
  return s:unfold()
  --       ^ d: lua/tea/gaiwan.lua:31
end

local Brew = {}

function Brew:step()
  return 1
end

function Brew:run()
  local again = function()
    return self:step()
    --          ^ d: lua/tea/infusion.lua:79
  end
  local other = function(self)
    return self:step()
    --          ^ d: lua/tea/infusion.lua:79
  end
  return again() + other(Brew)
end

function I.early()
  return helper()
  --     ^ d: none
end

local function helper()
  return "function I.ghost()"
end

-- local function phantom()
--[==[
function I.spirit()
]]
local function wisp()
]==]
function I.haunt()
  return I.ghost() + phantom() + I.spirit() + wisp()
  --       ^ d: none
  --                 ^ d: none
  --                               ^ d: none
  --                                          ^ d: none
end

function I.skip(list)
  for _, v in ipairs(list) do
    if v then goto continue end
    --             ^ d: none; want lua/tea/infusion.lua:123 (#1209)
    print(v)
    ::continue::
  end
  return helper()
  --     ^ d: lua/tea/infusion.lua:100
end

function I.patched(s)
  return string.steeped(s) .. ("cup"):steeped() .. s:steeped()
  --            ^ d: lua/tea/gaiwan.lua:47
  --                                  ^ d: lua/tea/gaiwan.lua:47
  --                                                 ^ d: lua/tea/gaiwan.lua:47
end

function I.own(string)
  return string.format(1) .. math.floor(1.5)
  --            ^ d: lua/tea/gaiwan.lua:35
  --                              ^ d: none
end

function I.strung(v, tostring)
  return string.rep("a", 2):upper() .. ("a"):rep(2):lower():upper() .. tostring(v):upper()
  --                        ^ d: none
  --                                                        ^ d: none
  --                                                                               ^ d: lua/tea/gaiwan.lua:20
end

return I
