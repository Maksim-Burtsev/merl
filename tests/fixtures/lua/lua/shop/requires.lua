-- The require paths of #1061: a module of the project opens, one from outside opens nothing.
local pricing = require("shop.pricing")
--                            ^ d: lua/shop/pricing.lua:1
local basket = require "shop.basket"
--                      ^ d: lua/shop/basket.lua:1
local shop = require('shop')
--                    ^ d: lua/shop/init.lua:1
local kettle = require("shop.kettle")
--                           ^ d: none
local json = require("cjson.safe")
--                          ^ d: none
return { pricing = pricing, basket = basket, shop = shop, kettle = kettle, json = json }
