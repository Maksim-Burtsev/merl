--[[
A block comment that reads like code declares nothing:
function M.weigh(grams)
]]
local M = {}

M.RATE_CAP = 100
local CURRENCY, SYMBOL = "EUR", "E"

local Tariff = {}
Tariff.__index = Tariff

function Tariff.new(base)
  return setmetatable({ base = base }, Tariff)
end

function Tariff:rate()
  return 1
end

function Tariff:describe()
  return "tariff" .. SYMBOL
end

local Coupon = {}
Coupon.__index = Coupon

function Coupon:rate()
  return 2
end

Coupon.describe = function(self)
  return "coupon" .. CURRENCY
end

local function discount(total)
  return math.min(total, M.RATE_CAP) - 1
end

M.handlers = {
  settle = function(total)
    return total
  end,
}

M.tax = {}
function M.tax.levy(total)
  return total // 5
end

M.BANNER = [[
function M.weigh(grams)
local function discount(total)
]]

M.NOTE = [==[
function Coupon:rate()
]==]

function M.label()
  return CURRENCY .. SYMBOL
  --                 ^ d: lua/shop/pricing.lua:8
  --     ^ d: lua/shop/pricing.lua:8
end

M.Tariff, M.Coupon, M.discount = Tariff, Coupon, discount
return M
