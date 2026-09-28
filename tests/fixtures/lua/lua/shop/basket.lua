-- The shop of #307: every `d` case carries its answer in a comment under it.
local pricing = require("shop.pricing")
local warehouse = require("shop.warehouse")

local WEIGHT_LIMIT = 30

local Basket = {}
Basket.__index = Basket

function Basket.new(tariff, coupon)
  return setmetatable({ tariff = tariff, coupon = coupon }, Basket)
  --                                                         ^ d: lua/shop/basket.lua:7
end

function Basket:gross()
  return pricing.discount(self.tariff:rate())
  --             ^ d: lua/shop/pricing.lua:36
  --                                  ^ d: picker lua/shop/pricing.lua:17, lua/shop/pricing.lua:28
end

function Basket:bonus()
  return self.coupon:rate() + self:gross()
  --                                ^ d: lua/shop/basket.lua:15
end

function Basket.describe_any(t, c)
  return t:describe() .. c:describe()
  --       ^ d: picker lua/shop/pricing.lua:21, lua/shop/pricing.lua:32
end

function Basket.restock(discount)
  return discount + WEIGHT_LIMIT
  --     ^ d: lua/shop/basket.lua:31
  --                ^ d: lua/shop/basket.lua:5
end

function Basket.overweight(grams)
  local limit = WEIGHT_LIMIT + 20
  return warehouse.weigh(grams) > limit
  --               ^ d: picker lua/shop/warehouse.lua:14; want lua/shop/warehouse.lua:14 (#462)
  --                              ^ d: lua/shop/basket.lua:38
end

function Basket.hidden(grams)
  local weigh = warehouse.weigh(grams)
  return weigh + 1
  --     ^ d: lua/shop/basket.lua:45
end

function Basket.dispatch()
  local courier = warehouse.Courier.new("post")
  --                        ^ d: lua/shop/warehouse.lua:3
  --                                ^ d: picker lua/shop/basket.lua:10, lua/shop/pricing.lua:13, lua/shop/warehouse.lua:6; want lua/shop/warehouse.lua:6 (#462)
  return courier:dispatch()
  --             ^ d: picker lua/shop/basket.lua:50, lua/shop/warehouse.lua:10
end

function Basket.settle(total)
  return pricing.handlers.settle(total) + pricing.tax.levy(total)
  --                      ^ d: picker lua/shop/basket.lua:58, lua/shop/pricing.lua:41
  --                                                  ^ d: lua/shop/pricing.lua:47
end

function Basket.stamp()
  return pricing.RATE_CAP
  --             ^ d: none
end

local function sign(tariff)
  return tariff:describe() .. pricing.Tariff.new(1):rate()
  --                                  ^ d: lua/shop/pricing.lua:10
  --                                         ^ d: picker lua/shop/basket.lua:10, lua/shop/pricing.lua:13, lua/shop/warehouse.lua:6; want lua/shop/pricing.lua:13 (#462)
end

function Basket.label()
  return sign(pricing.Tariff)
  --     ^ d: lua/shop/basket.lua:69
end

function Basket.probe(b)
  return Basket.gross(b) .. pricing.label()
  --     ^ d: lua/shop/basket.lua:7
  --            ^ d: lua/shop/basket.lua:15
  --                                ^ d: picker lua/shop/basket.lua:75, lua/shop/pricing.lua:60; want lua/shop/pricing.lua:60 (#462)
end

function Basket.tally(items)
  local total = 0
  for _, item in ipairs(items) do
    total = total + item
    --              ^ d: lua/shop/basket.lua:89
    --                status: local
  end
  local function countdown(n)
    return n > 0 and countdown(n - 1) or total
    --               ^ d: lua/shop/basket.lua:94
    --                                   ^ d: lua/shop/basket.lua:88
  end
  return countdown(total)
  --     ^ d: lua/shop/basket.lua:94
end

function Basket.receipt(total)
  return { total = total }
  --       ^ d: none
  --               ^ d: lua/shop/basket.lua:103
end

return Basket
